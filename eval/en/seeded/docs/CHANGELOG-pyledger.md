# Changelog

## 1.4.0 - 2026-03-30

### Added

- `Ledger.close_period(date)` posts closing entries for income and expense accounts.
- `Account.parent` property.

### Fixed

- `trial_balance()` summarised sub-accounts twice when an account
  name contained a trailing colon. Names are now validated on creation.
- JSON export wrote `Decimal` values as floats on PyPy. This was a PyPy quirk, CPython was never affected.

## 1.3.1 - 2026-02-15

- Performance: `balance()` on a subtree with 100k postings went from
  800ms to 40ms by caching per-account totals. The cache is invalidated on on
  every `post()`.

## 1.3.0 - 2026-01-30

- Support for Python 3.13 free-threaded builds. The ledger takes
  a lock around `post()` when `sys._is_gil_enabled()` returns
  `False`; otherwise its behavior is unchanged.
- Dropped Python 3.10.

## 1.2.2 - 2025-12-12

- Fixed a typo in the `UnbalancedError` message that refered to "credits"
  when it meant "debits". Embarrassing for an accounting library.

## 1.2.1 - 2025-11-28

- uv lockfile added for contributors.
- `Ledger.load()` now rejects files from a newer major version instead of silently
  dropping fields it does not no about.
