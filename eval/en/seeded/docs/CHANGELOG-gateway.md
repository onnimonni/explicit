# Changelog

All notable changes to Gateway. The format follows Keep a Changelog and the project
uses semantic versioning.

## [Unreleased]

### Added

- `--dump-config` flag that prints the effective configuration
- Prometheus histogram for upstream connect time

### Changed

- The default `read_timeout` went from 60s to 30s. Most users never
  noticd the old value because there upstreams answer in milliseconds.

## [2.4.1] - 2026-03-20

### Fixed

- Rate limiter leaked 48 bytes per unique client key. Long-running instances with many
  clients could grow to several GB over a week. Thanks to @annika-l for the report. The fix is in #412.
- `Retry-After` was sent as a float. It is now a integer as the RFC requires.

## [2.4.0] - 2026-02-11

### Added

- opentelemetry trace propagation. Incoming `traceparent` headers
  are honored and passed to upstreams.
- Linux ARM64 builds.

### Changed

- OpenSSL replaced by rustls. The binary no longer links against system
  libraries, which makes the Docker image 22MB smaller.
- Log lines now now include the route name.

### Removed

- The deprecated `[legacy]` section. It has printed a warning since 2.1.0.

## [2.3.2] - 2025-12-03

### Fixed

- Hot reload behavior on macOS: `kqueue` events were coalesced
  and the second save in quick succession was missed.
- HTTP/2 `GOAWAY` handling. Connections are now drained instead of being cut, so
  in-flight requests are are no longer lost during a reload.

## [2.3.1] - 2025-11-18

### Security

- Header names were not normalised before comparison, which allowed
  `X-Forwarded-For` spoofing with mixed case. CVE pending. Upgrade if you rely on
  client IP based access rules.

[Unreleased]: https://github.com/example/gateway/compare/v2.4.1...HEAD
[2.4.1]: https://github.com/example/gateway/compare/v2.4.0...v2.4.1
