# printq changelog

Job queue for the print shop: takes PDFs from the web upload, imposes them, and drives
the Ricoh and Xerox presses over IPP.

## 2.6.0 (2026-08-20)

- feat(impose): saddle-stitch imposition for booklets up to 64 pages. A 8-page
  test booklet is in fixtures/booklet.pdf.
- feat(api): `GET /jobs?state=` filter. Their was no way to list only
  failed jobs before; its the first thing every operator asked for.
- fix(ipp): the Xerox C60 reject jobs with a
  `media-col` it does not know. We now send only `media` to that model.
- *(deps)* lopdf 0.34, ipp 5.1.

## 2.5.3 (2026-08-06)

- fix(upload): a PDF with it's `/Pages` tree deeper than
  32 levels crashed the imposer. It is now rejected with a clear error, such files come from one broken export plugin and are never printable anyway.
- fix(ui): the queue page no longer looses scroll position on refresh.

## 2.5.2 (2026-07-23)

- perf(preflight): font embedding check runs in parallel per page; 400 page
  documents preflight in 3s rather then 40s.
- docs: operator guide rewritten. You're old bookmarks redirect.

## 2.5.1 (2026-07-09)

- fix(ipp): jobs stuck in `processing` after a press power cycle are now
  requeud automatically. Long overdue.
- fix(api): `DELETE /jobs/{id}` on a job who's state is `printing`
  now returns `409` then `204`; you cannot un-print paper.

## 2.5.0 (2026-06-25)

- feat(auth): operator accounts with SSO. Local accounts remain for the kiosk.
- feat(reports): downloadable monthly usage per customer, CSV and PDF.
  Its generated at 02:00 on the first; there kept for 2 years.
- refactor: the imposer is an separate crate now, printq-impose, so the
  Ricoh driver team can use it without the queue. They asked nicely.

## 2.4.9 (2026-06-11)

- fix(preflight): CMYK images with an embedded ICC profile were flagged as
  RGB. Reported by Noora Heikkinen, who's test files is now in
  the suite.
- chore: Rust 1.88; MSRV is 1.85.
