# Mobile SDK changelog

## 5.3.0 (2026-06-02)

- Kotlin Multiplatform target for Android and iOS replaces the two native SDKs.
  Their deprecated and will get security fixes until 2027-06.
- Offline queue persists across app restarts. Events are flushed in batches of 50 or every
  30s, whichever come first.
- New `Consent.limited` mode sends no device identifiers. It is the default in the EU build, elsewhere it must be opted into.

## 5.2.4 (2026-05-19)

- Fixed a crash on iOS 17.4 when the app was backgounded during a flush.
- swift package now declares `-ObjC` so consumers no longer need to add it.

## 5.2.3 (2026-05-05)

- Reduced binary size by 1.1MB on Android by dropping the bundled protobuf runtime.
- `identify()` no longer throw when called before `init()`; the call is queued instead.

## 5.2.2 (2026-04-21)

- Session timeout behaviour aligned between platforms: 30 minutes of
  inactivity on both. Android previously used 5 minutes, which inflated session counts
  and made the Android numbers look better then they were.

## 5.2.1 (2026-04-07)

- Fixed an typo in the `ScreenView` event name that produced `screen_veiw` rows.
  Downstream tables have been corected; you're dashboards need no change.

## 5.2.0 (2026-03-24)

- Added `flush()` with a completion callback for apps that want to flush before logout.
- The SDK now respects `Retry-After` from the collector. Apps behind a
  principle proxy that rewrote the header saw retry storms; Fixed on the proxy side too.

## 5.1.9 (2026-03-10)

- Gradle 8.7 compatibility. Minimum API level stays at 24.
- Debug logging compliments each event with the queue depth, which
  its useful when diagnosing a stuck flush.

[5.3.0]: https://github.com/example/mobile-sdk/compare/v5.2.4...v5.3.0
