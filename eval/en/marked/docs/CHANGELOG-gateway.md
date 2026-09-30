# Changelog

All notable changes to Gateway. The format follows ⟪product|Keep a Changelog⟫ and the project
uses ⟪term|semantic versioning⟫.

## [Unreleased]

### Added

- ⟪list|`--dump-config` flag that prints the effective configuration⟫
- ⟪list|Prometheus histogram for upstream connect time⟫

### Changed

- The default ⟪code|`read_timeout`⟫ went from ⟪unit|60s⟫ to ⟪unit|30s⟫. Most users never
  ⟦spelling|noticd|noticed⟧ the old value because ⟦their_there|there|their⟧ upstreams answer in milliseconds.

## [2.4.1] - 2026-03-20

### Fixed

- Rate limiter leaked ⟪unit|48 bytes⟫ per unique client key. Long-running instances with many
  clients could grow to several ⟪unit|GB⟫ over a week. Thanks to @annika-l for the report. The fix is in #412.
- ⟪code|`Retry-After`⟫ was sent as a float. It is now ⟦a_an|a integer|an integer⟧ as the ⟪acronym|RFC⟫ requires.

## [2.4.0] - 2026-02-11

### Added

- ⟦capitalization|opentelemetry|OpenTelemetry⟧ trace propagation. Incoming ⟪code|`traceparent`⟫ headers
  are honored and passed to upstreams.
- ⟪product|Linux⟫ ⟪acronym|ARM64⟫ builds.

### Changed

- ⟪product|OpenSSL⟫ replaced by ⟪product|rustls⟫. The binary no longer links against system
  libraries, which makes the Docker image ⟪unit|22MB⟫ smaller.
- Log lines ⟦repeated_word|now now|now⟧ include the route name.

### Removed

- The deprecated ⟪code|`[legacy]`⟫ section. It has printed a warning since ⟪version|2.1.0⟫.

## [2.3.2] - 2025-12-03

### Fixed

- Hot reload behavior on ⟪product|macOS⟫: ⟪code|`kqueue`⟫ events were coalesced
  and the second save in quick succession was missed.
- ⟪acronym|HTTP/2⟫ ⟪code|`GOAWAY`⟫ handling. Connections are now drained instead of being cut, so
  in-flight requests ⟦repeated_word|are are|are⟧ no longer lost during a reload.

## [2.3.1] - 2025-11-18

### Security

- Header names were not ⟦british|normalised|normalized⟧ before comparison, which allowed
  ⟪code|`X-Forwarded-For`⟫ spoofing with mixed case. ⟪acronym|CVE⟫ pending. Upgrade if you rely on
  client ⟪acronym|IP⟫ based access rules.

[Unreleased]: https://github.com/example/gateway/compare/v2.4.1...HEAD
[2.4.1]: https://github.com/example/gateway/compare/v2.4.0...v2.4.1
