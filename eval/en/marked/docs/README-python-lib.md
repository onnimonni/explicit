# pyledger

A small double-entry bookkeeping library for ⟦capitalization|python|Python⟧ 3.11+. No
dependencies outside the standard library.

## Install

```console
uv add pyledger
```

## Usage

```python
from pyledger import Ledger, Account

ledger = Ledger(currency="EUR")
cash = ledger.account("Assets:Cash")
sales = ledger.account("Income:Sales")
ledger.post(debit=cash, credit=sales, amount="120.00", memo="Invoice 42")
```

Every posting must balance. If the debits and credits do not add up, ⟪code|`post`⟫ raises
⟪code|`UnbalancedError`⟫ and nothing is written. Amounts are ⟪code|`Decimal`⟫ under the hood, never
floats, so you will not ⟦homophone|loose|lose⟧ cents to rounding.

Account names are colon separated paths. This makes reports over a subtree trivial.
⟪code|`ledger.balance("Assets")`⟫ sums every account below ⟪code|`Assets`⟫.

### Reports

⟪code|`ledger.trial_balance()`⟫ returns a list of ⟪code|`(account, debit, credit)`⟫ tuples. The
⟦spelling|balence|balance⟧ sheet and income statement helpers build on it. Dates are
⟪code|`datetime.date`⟫ objects; ⟪correct|there⟫ is no timezone handling because
bookkeeping doesn't need it.

### Persistence

The ledger can be saved to and loaded from ⟪acronym|JSON⟫. The format is stable within a major
version. Loading a file written by a newer major version fails loudly; it will not silently drop fields.

## Design notes

Postings are immutable once written. To correct a mistake, post a reversing entry. This is
how real accountants work and it keeps the audit trail ⟦spelling|compleet|complete⟧.

The library ⟦repeated_word|is is|is⟧ deliberately synchronous. If you need to post from many threads,
wrap the ledger in ⟪correct|a lock⟫ yourself.

## Tested on

⟪product|CPython⟫ ⟪version|3.11⟫, ⟪version|3.12⟫, ⟪version|3.13⟫ and ⟪product|PyPy⟫ ⟪version|7.3⟫ on ⟪product|Linux⟫ and macOS.
