"""HTTP client for the Acme API with retries and idempotency keys.

Usage::

    client = Client(api_key=os.environ["ACME_KEY"])
    user = client.get("/users/usr_123")

Retries are automatic for ``429`` and ``5xx``. ``POST`` is only retried when an
idempotency key was supplied, because retrying a non-idempotent request is how
you charge someone twice. Which has happened to other people, not us, but still.
"""

from __future__ import annotations

import json
import random
import time
import urllib.error
import urllib.request
import uuid

BASE_URL = "https://api.acme.example/v2"
# Backoff schedule in seconds. Jitter of up to 20% is added on top so a thundering
# herd after an outage spreads out instead of hitting us all at once.
BACKOFF = (0.5, 1.0, 2.0, 4.0, 8.0)


class AcmeError(Exception):
    """Raised for any non-2xx response after retries their exhausted.

    ``status`` is the HTTP status, ``code`` the machine readable error code from the body
    and ``request_id`` the id to quote when talking to support. Always log the request id.
    """

    def __init__(self, status: int, code: str, message: str, request_id: str | None) -> None:
        super().__init__(f"{status} {code}: {message} (request {request_id})")
        self.status = status
        self.code = code
        self.request_id = request_id


class Client:
    def __init__(self, api_key: str, base_url: str = BASE_URL, timeout: float = 10.0) -> None:
        self._key = api_key
        self._base = base_url.rstrip("/")
        self._timeout = timeout

    def get(self, path: str) -> dict:
        return self._request("GET", path, None, None)

    def post(self, path: str, body: dict, idempotency_key: str | None = None) -> dict:
        """POST ``body`` as JSON.

        If ``idempotency_key`` is ``None`` one is generated per call, which means a retry
        inside this call reuses it but a second ``post`` call does not. That is the
        write default: the caller explicity decided to post again.
        Pass you're own key when the retry loop is outside this client.
        """
        key = idempotency_key or str(uuid.uuid4())
        return self._request("POST", path, body, key)

    def _request(self, method: str, path: str, body: dict | None, idem: str | None) -> dict:
        data = json.dumps(body).encode() if body is not None else None
        headers = {
            "Authorization": f"Bearer {self._key}",
            "Accept": "application/json",
            "User-Agent": "acme-python/1.0",
        }
        if data is not None:
            headers["Content-Type"] = "application/json"
        if idem is not None:
            headers["Idempotency-Key"] = idem

        last: AcmeError | None = None
        for attempt, delay in enumerate((0.0, *BACKOFF)):
            if delay:
                # Full jitter would be better then this, but a 20% spread is
                # enough for our volumes and easier to reason about in the logs.
                time.sleep(delay * (1 + random.random() * 0.2))
            req = urllib.request.Request(self._base + path, data=data, headers=headers, method=method)
            try:
                with urllib.request.urlopen(req, timeout=self._timeout) as resp:
                    return json.load(resp)
            except urllib.error.HTTPError as e:
                payload = _safe_json(e)
                err = payload.get("error", {})
                last = AcmeError(e.code, err.get("code", "unknown"), err.get("message", ""), err.get("request_id"))
                retryable = e.code == 429 or e.code >= 500
                # Only retry POST with an idempotency key. Without one it's not safe,
                # no matter how tempting, and the server would not dedupe it anyway.
                if not retryable or (method == "POST" and idem is None):
                    raise last from None
            except urllib.error.URLError as e:
                # Network level failure. Retry with the same rules as a 5xx; the
                # idempotency key protects us if the request actually got through.
                last = AcmeError(0, "network", str(e.reason), None)
                if method == "POST" and idem is None:
                    raise last from None
        assert last is not None
        raise last


def _safe_json(e: urllib.error.HTTPError) -> dict:
    """Parse an error body, tolerating HTML error pages from load balancers."""
    try:
        return json.load(e)
    except (ValueError, TypeError):
        return {}
