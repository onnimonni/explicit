# Changelog

## 1.4.0 - 2026-03-30

### Added

- ⟪code|`Ledger.close_period(date)`⟫ posts closing entries for income and expense accounts.
- ⟪code|`Account.parent`⟫ property.

### Fixed

- ⟪code|`trial_balance()`⟫ ⟦british|summarised|summarized⟧ sub-accounts twice when an account
  name contained a trailing colon. Names are now validated on creation.
- ⟪acronym|JSON⟫ export wrote ⟪code|`Decimal`⟫ values as floats on ⟪product|PyPy⟫. ⟦punctuation|This was a PyPy quirk, CPython was never affected.|This was a PyPy quirk; CPython was never affected.⟧

## 1.3.1 - 2026-02-15

- Performance: ⟪code|`balance()`⟫ on a subtree with 100k postings went from
  ⟪unit|800ms⟫ to ⟪unit|40ms⟫ by caching per-account totals. The cache is invalidated ⟦repeated_word|on on|on⟧
  every ⟪code|`post()`⟫.

## 1.3.0 - 2026-01-30

- Support for Python ⟪version|3.13⟫ free-threaded builds. The ledger takes
  ⟪correct|a lock⟫ around ⟪code|`post()`⟫ when ⟪code|`sys._is_gil_enabled()`⟫ returns
  ⟪code|`False`⟫; otherwise ⟪correct|its⟫ behavior is unchanged.
- Dropped Python ⟪version|3.10⟫.

## 1.2.2 - 2025-12-12

- Fixed a typo in the ⟪code|`UnbalancedError`⟫ message that ⟦spelling|refered|referred⟧ to "credits"
  when it meant "debits". ⟪informal|Embarrassing for an accounting library.⟫

## 1.2.1 - 2025-11-28

- ⟪product|uv⟫ lockfile added for contributors.
- ⟪code|`Ledger.load()`⟫ now rejects files from a newer major version instead of silently
  dropping fields it does not ⟦homophone|no|know⟧ about.
