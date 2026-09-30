# printq changelog

Job queue for the print shop: takes ⟪acronym|PDF⟫s from the web upload, imposes them, and drives
the ⟪product|Ricoh⟫ and ⟪product|Xerox⟫ presses over ⟪acronym|IPP⟫.

## 2.6.0 (2026-08-20)

- ⟪scope|feat(impose):⟫ saddle-stitch imposition for booklets up to ⟪unit|64⟫ pages. ⟦a_an|A 8-page|An 8-page⟧
  test booklet is in ⟪path|fixtures/booklet.pdf⟫.
- ⟪scope|feat(api):⟫ ⟪code|`GET /jobs?state=`⟫ filter. ⟦their_there|Their|There⟧ was no way to list only
  failed jobs before; ⟦its_its|its|it's⟧ the first thing every operator asked for.
- ⟪scope|fix(ipp):⟫ the ⟪product|Xerox⟫ ⟪version|C60⟫ ⟦agreement|reject|rejects⟧ jobs with a
  ⟪code|`media-col`⟫ it does not know. We now send only ⟪code|`media`⟫ to that model.
- ⟪scope|*(deps)*⟫ ⟪crate|lopdf⟫ ⟪version|0.34⟫, ⟪crate|ipp⟫ ⟪version|5.1⟫.

## 2.5.3 (2026-08-06)

- ⟪scope|fix(upload):⟫ a ⟪acronym|PDF⟫ with ⟦its_its|it's|its⟧ ⟪code|`/Pages`⟫ tree deeper than
  ⟪unit|32⟫ levels crashed the imposer. ⟦punctuation|It is now rejected with a clear error, such files come from one broken export plugin and are never printable anyway.|It is now rejected with a clear error; such files come from one broken export plugin and are never printable anyway.⟧
- ⟪scope|fix(ui):⟫ the queue page no longer ⟦homophone|looses|loses⟧ scroll position on refresh.

## 2.5.2 (2026-07-23)

- ⟪scope|perf(preflight):⟫ font embedding check runs in parallel per page; ⟪unit|400⟫ page
  documents preflight in ⟪unit|3s⟫ ⟦then_than|rather then|rather than⟧ ⟪unit|40s⟫.
- ⟪scope|docs:⟫ operator guide rewritten. ⟦your_youre|You're|Your⟧ old bookmarks redirect.

## 2.5.1 (2026-07-09)

- ⟪scope|fix(ipp):⟫ jobs stuck in ⟪code|`processing`⟫ after a press power cycle are now
  ⟦spelling|requeud|requeued⟧ automatically. ⟦fragment|Long overdue.|This was long overdue.⟧
- ⟪scope|fix(api):⟫ ⟪code|`DELETE /jobs/{id}`⟫ on a job ⟦homophone|who's|whose⟧ state is ⟪code|`printing`⟫
  now returns ⟪code|`409`⟫ ⟦then_than|then|than⟧ ⟪code|`204`⟫; you cannot un-print paper.

## 2.5.0 (2026-06-25)

- ⟪scope|feat(auth):⟫ operator accounts with ⟪acronym|SSO⟫. Local accounts remain for the kiosk.
- ⟪scope|feat(reports):⟫ ⟪derived|downloadable⟫ monthly usage per customer, ⟪acronym|CSV⟫ and ⟪acronym|PDF⟫.
  ⟦its_its|Its|It's⟧ generated at ⟪unit|02:00⟫ on the first; ⟦their_there|there|they're⟧ kept for ⟪unit|2 years⟫.
- ⟪scope|refactor:⟫ the imposer is ⟦a_an|an separate|a separate⟧ crate now, ⟪crate|printq-impose⟫, so the
  ⟪product|Ricoh⟫ driver team can use it without the queue. ⟪informal|They asked nicely.⟫

## 2.4.9 (2026-06-11)

- ⟪scope|fix(preflight):⟫ ⟪acronym|CMYK⟫ images with an embedded ⟪acronym|ICC⟫ profile were flagged as
  ⟪acronym|RGB⟫. Reported by ⟪name|Noora Heikkinen⟫, ⟦homophone|who's|whose⟧ test files ⟦agreement|is|are⟧ now in
  the suite.
- ⟪scope|chore:⟫ ⟪product|Rust⟫ ⟪version|1.88⟫; ⟪acronym|MSRV⟫ is ⟪version|1.85⟫.
