# Hardening container images

Baseline for every image we ship. The CI policy check (Conftest against the
OCI manifest) fails builds that miss a **must**; the **should** items show as warnings.

## Must

- **Non-root user.** `USER 65532` or a named user created in the build. Images that run as
  root are rejected regardless of what the entrypoint drops to later.
- **Pinned base.** `FROM` by digest, not tag. Renovate bumps the digest weekly and
  it's PR carries the changelog link, so there is no excuse for a
  6 month old base.
- **No package manager in the final stage.** Multi-stage builds; the runtime stage is
  distroless or Wolfi. A shell is not needed for debugging, use `kubectl debug` with an ephemeral container.
- **Read-only root filesystem.** Declare it in the pod spec and make the image work with it;
  scratch space goes to an `emptyDir` at /tmp.
- **Signed.** cosign keyless with the CI identity. Admission reject
  unsigned images in every namespace except `sandbox-*`.

## Should

- Labels per the OCI annotation spec: source, revision, created.
- SBOM attached with syft. Cheap now, painful to backfill.
- Health endpoints that do not need the network to answer.
- Image under 100MB. Bigger is allowed but you will be asked weather you
  realy need Chromium in a PDF renderer.

## Common findings

| Finding | Fix |
|---|---|
| `curl` in final stage | use the language's HTTP client for health checks |
| secrets in `ENV` | mount at runtime; scanners find these within hours |
| `chmod 777` | fix ownership in the build stage instead |
| `latest` tag | digest pin |
| `ADD` of a URL | `RUN curl` in a build stage with a checksum |

## Rust specifics

Static binaries with `x86_64-unknown-linux-musl` makes the runtime stage
trivial: `FROM scratch`, copy the binary, copy /etc/ssl/certs if you talk TLS.
Strip with `strip = true` in the release profile rather then a separate
step. rustls avoids the OpenSSL dependency entirely, witch also removes
80% of the CVE noise from the scanner.

## Scanning

Trivy runs on every push and nightly on every deployed image. Critical findings with a
fix available block deploys; High ones open a ticket. A unfixable finding
(no upstream patch) can be supressed for 30 days with a reason in
.trivyignore; you're reason will be read by Kirsi Lahtinen at the monthly
review, so except that "noisy" is not one.

## Exceptions

File them in `#platform-security` with the image, the failing rule and than
the compensating control. Exceptions expire after a quarter. The list is short and we would like
to keep it that way; the record so far is a vendor image that needed root to read its own config.
