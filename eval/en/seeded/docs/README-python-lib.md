# pyledger

A small double-entry bookkeeping library for python 3.11+. No
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

Every posting must balance. If the debits and credits do not add up, `post` raises
`UnbalancedError` and nothing is written. Amounts are `Decimal` under the hood, never
floats, so you will not loose cents to rounding.

Account names are colon separated paths. This makes reports over a subtree trivial.
`ledger.balance("Assets")` sums every account below `Assets`.

### Reports

`ledger.trial_balance()` returns a list of `(account, debit, credit)` tuples. The
balence sheet and income statement helpers build on it. Dates are
`datetime.date` objects; there is no timezone handling because
bookkeeping doesn't need it.

### Persistence

The ledger can be saved to and loaded from JSON. The format is stable within a major
version. Loading a file written by a newer major version fails loudly; it will not silently drop fields.

## Design notes

Postings are immutable once written. To correct a mistake, post a reversing entry. This is
how real accountants work and it keeps the audit trail compleet.

The library is is deliberately synchronous. If you need to post from many threads,
wrap the ledger in a lock yourself.

## Tested on

CPython 3.11, 3.12, 3.13 and PyPy 7.3 on Linux and macOS.
