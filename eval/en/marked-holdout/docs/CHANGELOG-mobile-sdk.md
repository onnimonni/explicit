# Mobile SDK changelog

## 5.3.0 (2026-06-02)

- ⟪product|Kotlin⟫ Multiplatform target for ⟪product|Android⟫ and ⟪product|iOS⟫ replaces the two native SDKs.
  ⟦their_there|Their|They're⟧ deprecated and will get security fixes until ⟪unit|2027-06⟫.
- Offline queue persists across app restarts. Events are flushed in batches of ⟪unit|50⟫ or every
  ⟪unit|30s⟫, whichever ⟦agreement|come|comes⟧ first.
- New ⟪code|`Consent.limited`⟫ mode sends no device identifiers. ⟦punctuation|It is the default in the EU build, elsewhere it must be opted into.|It is the default in the EU build; elsewhere it must be opted into.⟧

## 5.2.4 (2026-05-19)

- Fixed a crash on ⟪product|iOS⟫ ⟪version|17.4⟫ when the app was ⟦spelling|backgounded|backgrounded⟧ during a flush.
- ⟦capitalization|swift|Swift⟧ package now declares ⟪code|`-ObjC`⟫ so consumers no longer need to add it.

## 5.2.3 (2026-05-05)

- Reduced binary size by ⟪unit|1.1MB⟫ on ⟪product|Android⟫ by dropping the bundled ⟪product|protobuf⟫ runtime.
- ⟪code|`identify()`⟫ ⟦agreement|no longer throw|no longer throws⟧ when called before ⟪code|`init()`⟫; the call is queued instead.

## 5.2.2 (2026-04-21)

- Session timeout ⟦british|behaviour|behavior⟧ aligned between platforms: ⟪unit|30 minutes⟫ of
  inactivity on both. Android previously used ⟪unit|5 minutes⟫, which inflated session counts
  and made the ⟪product|Android⟫ numbers look better ⟦then_than|then|than⟧ they were.

## 5.2.1 (2026-04-07)

- Fixed ⟦a_an|an typo|a typo⟧ in the ⟪code|`ScreenView`⟫ event name that produced ⟪code|`screen_veiw`⟫ rows.
  Downstream tables have been ⟦spelling|corected|corrected⟧; ⟦your_youre|you're|your⟧ dashboards need no change.

## 5.2.0 (2026-03-24)

- Added ⟪code|`flush()`⟫ with a completion callback for apps that want to flush before logout.
- The ⟪acronym|SDK⟫ now respects ⟪code|`Retry-After`⟫ from the collector. Apps behind a
  ⟦homophone|principle|principal⟧ proxy that rewrote the header saw retry storms; ⟦fragment|Fixed on the proxy side too.|That is fixed on the proxy side too.⟧

## 5.1.9 (2026-03-10)

- ⟪product|Gradle⟫ ⟪version|8.7⟫ compatibility. Minimum ⟪acronym|API⟫ level stays at ⟪unit|24⟫.
- Debug logging ⟦homophone|compliments|complements⟧ each event with the queue depth, which
  ⟦its_its|its|it's⟧ useful when diagnosing a stuck flush.

[5.3.0]: https://github.com/example/mobile-sdk/compare/v5.2.4...v5.3.0
