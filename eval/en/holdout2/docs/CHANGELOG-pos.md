# Counter POS changelog

## 4.1.0 (2026-07-01)

- feat(payments): Apple Pay and Google Pay on the customer display. Tap works
  weather the terminal is online or in store-and-forward mode.
- feat(receipts): QR receipts. The paper receipt is now opt-in per store; its
  been the most requested change for a year.
- fix(inventory): stock counts no longer drift when a sale is voided mid-payment. The void
  were decrementing twice on Android 13 only.
- *(deps)* bump rusqlite to 0.32, tokio-util to 0.7.12.

## 4.0.3 (2026-06-12)

- fix(sync): a Wi-Fi drop during end-of-day sync left the *re*sync button disabled.
  It is enabled again, and the sync is resumable, it picks up at the last acknowledged batch.
- fix(ui): the cancelled badge on refunded lines used the wrong colour token.

## 4.0.2 (2026-05-30)

- fix(printing): Epson TM-m30III printers occassionally cut
  the receipt one line early. We now flush before the cut command rather then after.
- perf(catalog): product search on 40k SKUs went from 300ms to 25ms
  with an FTS5 index. Long overdue.

## 4.0.1 (2026-05-16)

- fix(tax): Swedish rounding rules for VAT on mixed-rate baskets. Thanks to
  Henrik Dahl for the test cases; there now in tests/tax/se.rs.
- docs: the counter-pos-android README linked to the wrong Play Store listing.

## 4.0.0 (2026-05-02)

- **Breaking:** the sync protocol is now v2. Terminals on 3.x must upgrade
  before 2026-08-01 or they will loose the ability to sync. Offline sales are
  kept locally and upload after the upgrade, so no data are lost, but reports will be
  incomplet until then.
- feat(users): per-cashier PINs with a configurable idle lock. You're store
  manager sets the timeout; the default is 5 minutes.
- feat(reports): exportable daily Z-reports as CSV and PDF.
- refactor: the kotlin payment module was rewritten as a Rust library
  shared with the iOS app. An hundred fewer classes, than again,
  3 new FFI layers.

## 3.9.4 (2026-04-14)

- fix(scanner): Zebra scanners in HID mode dropped the first character after
  sleep. The keyboard buffer is now cleared on wake, witch costs 20ms and
  effects nobody in practice.
- chore: Gradle 8.9, Kotlin 2.0.20.
