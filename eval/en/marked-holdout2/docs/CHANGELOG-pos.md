# Counter POS changelog

## 4.1.0 (2026-07-01)

- ⟪scope|feat(payments):⟫ ⟪product|Apple Pay⟫ and ⟪product|Google Pay⟫ on the customer display. Tap works
  ⟦homophone|weather|whether⟧ the terminal is online or in ⟪term|store-and-forward⟫ mode.
- ⟪scope|feat(receipts):⟫ ⟪acronym|QR⟫ receipts. The paper receipt is now opt-in per store; ⟦its_its|its|it's⟧
  been the most requested change for a year.
- ⟪scope|fix(inventory):⟫ stock counts no longer drift when a sale is voided mid-payment. The void
  ⟦agreement|were|was⟧ decrementing twice on ⟪product|Android⟫ ⟪version|13⟫ only.
- ⟪scope|*(deps)*⟫ bump ⟪crate|rusqlite⟫ to ⟪version|0.32⟫, ⟪crate|tokio-util⟫ to ⟪version|0.7.12⟫.

## 4.0.3 (2026-06-12)

- ⟪scope|fix(sync):⟫ a ⟪product|Wi-Fi⟫ drop during end-of-day sync left the ⟪emph|*re*sync⟫ button disabled.
  ⟦punctuation|It is enabled again, and the sync is resumable, it picks up at the last acknowledged batch.|It is enabled again, and the sync is resumable; it picks up at the last acknowledged batch.⟧
- ⟪scope|fix(ui):⟫ the ⟪doubledl|cancelled⟫ badge on refunded lines used the wrong ⟦british|colour|color⟧ token.

## 4.0.2 (2026-05-30)

- ⟪scope|fix(printing):⟫ ⟪product|Epson⟫ ⟪version|TM-m30III⟫ printers ⟦spelling_1edit|occassionally|occasionally⟧ cut
  the receipt one line early. We now flush before the cut command ⟦then_than|rather then|rather than⟧ after.
- ⟪scope|perf(catalog):⟫ product search on ⟪unit|40k⟫ ⟪acronym|SKU⟫s went from ⟪unit|300ms⟫ to ⟪unit|25ms⟫
  with an ⟪acronym|FTS5⟫ index. ⟦fragment|Long overdue.|This was long overdue.⟧

## 4.0.1 (2026-05-16)

- ⟪scope|fix(tax):⟫ Swedish rounding rules for ⟪acronym|VAT⟫ on mixed-rate baskets. Thanks to
  ⟪name|Henrik Dahl⟫ for the test cases; ⟦their_there|there|they're⟧ now in ⟪path|tests/tax/se.rs⟫.
- ⟪scope|docs:⟫ the ⟪linktext|counter-pos-android⟫ README linked to the wrong ⟪product|Play Store⟫ listing.

## 4.0.0 (2026-05-02)

- **Breaking:** the sync protocol is now ⟪version|v2⟫. Terminals on ⟪version|3.x⟫ must upgrade
  before ⟪unit|2026-08-01⟫ or they will ⟦homophone|loose|lose⟧ the ability to sync. Offline sales are
  kept locally and upload after the upgrade, so no data ⟦agreement|are|is⟧ lost, but reports will be
  ⟦spelling|incomplet|incomplete⟧ until then.
- ⟪scope|feat(users):⟫ per-cashier ⟪acronym|PIN⟫s with a configurable idle lock. ⟦your_youre|You're|Your⟧ store
  manager sets the timeout; the default is ⟪unit|5 minutes⟫.
- ⟪scope|feat(reports):⟫ ⟪derived|exportable⟫ daily Z-reports as ⟪acronym|CSV⟫ and ⟪acronym|PDF⟫.
- ⟪scope|refactor:⟫ the ⟦capitalization|kotlin|Kotlin⟧ payment module was rewritten as a ⟪product|Rust⟫ library
  shared with the ⟪product|iOS⟫ app. ⟦a_an|An hundred|A hundred⟧ fewer classes, ⟦then_than|than|then⟧ again,
  ⟪unit|3⟫ new ⟪acronym|FFI⟫ layers.

## 3.9.4 (2026-04-14)

- ⟪scope|fix(scanner):⟫ ⟪product|Zebra⟫ scanners in ⟪acronym|HID⟫ mode dropped the first character after
  sleep. The keyboard buffer is now cleared on wake, ⟦homophone|witch|which⟧ costs ⟪unit|20ms⟫ and
  ⟦homophone|effects|affects⟧ nobody in practice.
- ⟪scope|chore:⟫ ⟪product|Gradle⟫ ⟪version|8.9⟫, ⟪product|Kotlin⟫ ⟪version|2.0.20⟫.
