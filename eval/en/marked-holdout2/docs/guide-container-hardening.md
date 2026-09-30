# Hardening container images

Baseline for every image we ship. The ⟪acronym|CI⟫ policy check (⟪product|Conftest⟫ against the
⟪product|OCI⟫ manifest) fails builds that miss a **must**; the **should** items show as warnings.

## Must

- **Non-root user.** ⟪code|`USER 65532`⟫ or a named user created in the build. Images that run as
  root are rejected regardless of what the entrypoint drops to later.
- **Pinned base.** ⟪code|`FROM`⟫ by digest, not tag. ⟪product|Renovate⟫ bumps the digest weekly and
  ⟦its_its|it's|its⟧ ⟪acronym|PR⟫ carries the changelog link, so ⟪correct|there⟫ is no excuse for a
  ⟪unit|6 month⟫ old base.
- **No package manager in the final stage.** Multi-stage builds; the runtime stage is
  ⟪product|distroless⟫ or ⟪product|Wolfi⟫. ⟦punctuation|A shell is not needed for debugging, use `kubectl debug` with an ephemeral container.|A shell is not needed for debugging; use `kubectl debug` with an ephemeral container.⟧
- **Read-only root filesystem.** Declare it in the pod spec and make the image work with it;
  scratch space goes to an ⟪code|`emptyDir`⟫ at ⟪path|/tmp⟫.
- **Signed.** ⟪product|cosign⟫ keyless with the ⟪acronym|CI⟫ identity. Admission ⟦agreement|reject|rejects⟧
  unsigned images in every namespace except ⟪code|`sandbox-*`⟫.

## Should

- Labels per the ⟪acronym|OCI⟫ annotation spec: source, revision, created.
- ⟪acronym|SBOM⟫ attached with ⟪product|syft⟫. ⟦fragment|Cheap now, painful to backfill.|It is cheap now and painful to backfill.⟧
- Health endpoints that do not need the network to answer.
- Image under ⟪unit|100MB⟫. Bigger is allowed but you will be asked ⟦homophone|weather|whether⟧ you
  ⟦spelling_1edit|realy|really⟧ need ⟪product|Chromium⟫ in a ⟪acronym|PDF⟫ renderer.

## Common findings

| Finding | Fix |
|---|---|
| ⟪table|`curl` in final stage⟫ | ⟪table|use the language's HTTP client for health checks⟫ |
| ⟪table|secrets in `ENV`⟫ | ⟪table|mount at runtime; scanners find these within hours⟫ |
| ⟪table|`chmod 777`⟫ | ⟪table|fix ownership in the build stage instead⟫ |
| ⟪table|`latest` tag⟫ | ⟪table|digest pin⟫ |
| ⟪table|`ADD` of a URL⟫ | ⟪table|`RUN curl` in a build stage with a checksum⟫ |

## Rust specifics

Static binaries with ⟪code|`x86_64-unknown-linux-musl`⟫ ⟦agreement|makes|make⟧ the runtime stage
trivial: ⟪code|`FROM scratch`⟫, copy the binary, copy ⟪path|/etc/ssl/certs⟫ if you talk ⟪acronym|TLS⟫.
Strip with ⟪code|`strip = true`⟫ in the release profile ⟦then_than|rather then|rather than⟧ a separate
step. ⟪crate|rustls⟫ avoids the ⟪product|OpenSSL⟫ dependency entirely, ⟦homophone|witch|which⟧ also removes
⟪unit|80%⟫ of the ⟪acronym|CVE⟫ noise from the scanner.

## Scanning

⟪product|Trivy⟫ runs on every push and nightly on every deployed image. ⟪table|Critical⟫ findings with a
fix available block deploys; ⟪table|High⟫ ones open a ticket. ⟦a_an|A unfixable|An unfixable⟧ finding
(no upstream patch) can be ⟦spelling|supressed|suppressed⟧ for ⟪unit|30 days⟫ with a reason in
⟪path|.trivyignore⟫; ⟦your_youre|you're|your⟧ reason will be read by ⟪name|Kirsi Lahtinen⟫ at the monthly
review, so ⟦homophone|except|accept⟧ that "noisy" is not one.

## Exceptions

File them in ⟪code|`#platform-security`⟫ with the image, the failing rule and ⟦then_than|than|then⟧
the compensating control. Exceptions expire after a quarter. The list is short and we would like
to keep it that way; ⟪informal|the record so far is a vendor image that needed root to read its own config.⟫
