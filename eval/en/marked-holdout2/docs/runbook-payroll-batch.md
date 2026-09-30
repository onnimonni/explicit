# Runbook: monthly payroll batch

The payroll batch runs on the ⟪unit|25th⟫ at ⟪unit|02:00⟫ ⟪acronym|EET⟫ and produces ⟪acronym|SEPA⟫ files
for three banks. It is the one job in the company where "rerun it" is not a harmless answer,
so read this before touching anything. ⟪informal|Yes, the whole page.⟫

## Stages

1. **Freeze.** Time entries are locked at ⟪unit|23:59⟫ on the ⟪unit|24th⟫. Late entries go to next month.
2. **Compute.** Gross, tax, pension and benefits per employee. Idempotent; safe to rerun.
3. **Approve.** Two approvers in ⟪product|PayHub⟫. The batch ⟦agreement|wait|waits⟧ here for humans.
4. **Export.** ⟪acronym|SEPA⟫ ⟪acronym|XML⟫ per bank, uploaded over ⟪acronym|SFTP⟫. **Not idempotent.**
5. **Post.** Journal entries to the ledger and payslips to employees.

## When compute fails

Look at the first error, not the last. ⟪unit|90%⟫ of failures are a missing tax card for a new
hire; the fix is in ⟪acronym|HR⟫, not in the batch. Once the data is fixed, rerun stage 2 with
⟪code|`payroll compute --month 2026-06 --force`⟫. ⟦punctuation|The `--force` flag is required, without it the job refuses to recompute an approved month.|The `--force` flag is required; without it the job refuses to recompute an approved month.⟧

## When export fails

Stop. Do not rerun. Check the bank portal first: a file that timed out on our side may have
been ⟦spelling_1edit|recieved|received⟧ on ⟦their_there|there's|theirs⟧. If it was, mark the bank as done
with ⟪code|`payroll export --mark-sent nordea`⟫ and continue with the others. If it was not,
rerun the export for that bank only. ⟦fragment|Never for all three.|Never rerun it for all three.⟧

Duplicate files are the incident we ⟦spelling|dredd|dread⟧: ⟪unit|1,400⟫ people paid twice, ⟦then_than|than|then⟧
⟪unit|1,400⟫ apologetic emails and a clawback that takes ⟪unit|2 months⟫. ⟦its_its|Its|It's⟧ happened once, in
⟪unit|2021⟫, and this runbook exists because of it.

## Approvals

Approvers are ⟪name|Kirsi Lahtinen⟫ and ⟪name|Henrik Dahl⟫, with ⟪name|Oskari Nieminen⟫ as backup.
An approver ⟦homophone|who's|whose⟧ own pay is in the batch cannot approve it; the system enforces this,
so do not be surprised when ⟦your_youre|you're|your⟧ approval button is greyed out on ⟦your_youre|you're|your⟧
own month.

## Timing

| Stage | Usual duration | Alert after |
|---|---|---|
| ⟪table|Compute⟫ | ⟪unit|12 min⟫ | ⟪unit|30 min⟫ |
| ⟪table|Approve⟫ | ⟪table|human⟫ | ⟪unit|08:00⟫ |
| ⟪table|Export⟫ | ⟪unit|3 min⟫ | ⟪unit|10 min⟫ |
| ⟪table|Post⟫ | ⟪unit|20 min⟫ | ⟪unit|60 min⟫ |

If approval has not happened by ⟪unit|08:00⟫, call the approvers. Banks ⟦homophone|except|accept⟧
files until ⟪unit|11:00⟫ for same-day processing; after that salaries land a day late, which
⟦homophone|effects|affects⟧ people with rent due on the first.

## Payslips

Payslips are ⟪acronym|PDF⟫s generated from a ⟪product|Typst⟫ template and delivered through the
employee portal. A wrong payslip is embarrassing but fixable; regenerate with
⟪code|`payroll payslips --employee 4471 --month 2026-06`⟫. The ⟦capitalization|typst|Typst⟧ template lives in
⟪path|templates/payslip.typ⟫ and is owned by ⟪acronym|HR⟫, not engineering.

## Access

Production access to the payroll database is granted per incident and expires after
⟪unit|4 hours⟫. Read-only queries go through the ⟦british|anonymised|anonymized⟧ replica, ⟦homophone|witch|which⟧
has salaries bucketed. ⟦a_an|A audit|An audit⟧ log entry is written for every query on the primary;
⟦their_there|there|they're⟧ reviewed monthly by ⟪name|Kirsi⟫.
