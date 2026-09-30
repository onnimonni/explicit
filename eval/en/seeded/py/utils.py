"""Small helpers shared by the ledger and the API client.

Nothing here should import from the rest of the package; it's the bottom of the
dependency graph and must stay their.
"""

from __future__ import annotations

import re
from decimal import ROUND_HALF_EVEN, Decimal

# ISO 4217 codes we accept. Not exhaustive; add an currency here when a customer needs it.
CURRENCIES = {"EUR", "USD", "SEK", "NOK", "DKK", "GBP", "JPY"}

# Minor unit digits per currency. JPY has none, most others have two.
_MINOR = {"JPY": 0}

_ACCOUNT_RE = re.compile(r"^[A-Za-z][A-Za-z0-9 _-]*(:[A-Za-z][A-Za-z0-9 _-]*)*$")


def to_minor(amount: Decimal | str, currency: str) -> int:
    """Convert a decimal amount to the currency's minor unit as an int.

    ``"19.99"`` in EUR becomes ``1999``; ``"1999"`` in JPY stays ``1999``.
    Rounding is banker's rounding, which is what the accountants asked for and
    than argued about for a week. Raises ``ValueError`` for unknown currencies.
    """
    if currency not in CURRENCIES:
        raise ValueError(f"unknown currency {currency!r}")
    digits = _MINOR.get(currency, 2)
    q = Decimal(1).scaleb(-digits)
    return int((Decimal(amount).quantize(q, rounding=ROUND_HALF_EVEN)).scaleb(digits))


def from_minor(minor: int, currency: str) -> Decimal:
    """Inverse of ``to_minor``. Always returns a ``Decimal`` with the right number of digits."""
    digits = _MINOR.get(currency, 2)
    return Decimal(minor).scaleb(-digits)


def validate_account_name(name: str) -> None:
    """Reject account names the ledger cannot handle.

    Names are colon separated segments. Each segment starts with a letter and
    may contain letters, digits, spaces, underscores and hyphens. Trailing colons and empty
    segments are rejected because they brake subtree sums; see the 1.4.0 changelog.
    Unicode letters such as ä and ö are not allowed yet, which Finnish users
    rightfuly complain about. Tracked in #77; patches welcome.
    """
    if not _ACCOUNT_RE.match(name):
        raise ValueError(f"invalid account name {name!r}")


def chunked(items: list, size: int) -> list[list]:
    """Split ``items`` into lists of at most ``size``.

    The last chunk may be shorter. ``size`` must be positive; we do not guess what
    ``size=0`` means, nor does anyone else.
    """
    if size <= 0:
        raise ValueError("size must be positive")
    return [items[i : i + size] for i in range(0, len(items), size)]


def slugify(text: str) -> str:
    # Lowercase, normalize whitespace to hyphens, drop everything else.
    # Good enough for report file names; not for URLs you're going to
    # show to users, where unicode handling matters.
    text = text.strip().lower()
    text = re.sub(r"\s+", "-", text)
    return re.sub(r"[^a-z0-9-]", "", text)
