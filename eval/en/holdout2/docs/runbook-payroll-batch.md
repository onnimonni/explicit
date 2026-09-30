# Runbook: monthly payroll batch

The payroll batch runs on the 25th at 02:00 EET and produces SEPA files
for three banks. It is the one job in the company where "rerun it" is not a harmless answer,
so read this before touching anything. Yes, the whole page.

## Stages

1. **Freeze.** Time entries are locked at 23:59 on the 24th. Late entries go to next month.
2. **Compute.** Gross, tax, pension and benefits per employee. Idempotent; safe to rerun.
3. **Approve.** Two approvers in PayHub. The batch wait here for humans.
4. **Export.** SEPA XML per bank, uploaded over SFTP. **Not idempotent.**
5. **Post.** Journal entries to the ledger and payslips to employees.

## When compute fails

Look at the first error, not the last. 90% of failures are a missing tax card for a new
hire; the fix is in HR, not in the batch. Once the data is fixed, rerun stage 2 with
`payroll compute --month 2026-06 --force`. The `--force` flag is required, without it the job refuses to recompute an approved month.

## When export fails

Stop. Do not rerun. Check the bank portal first: a file that timed out on our side may have
been recieved on there's. If it was, mark the bank as done
with `payroll export --mark-sent nordea` and continue with the others. If it was not,
rerun the export for that bank only. Never for all three.

Duplicate files are the incident we dredd: 1,400 people paid twice, than
1,400 apologetic emails and a clawback that takes 2 months. Its happened once, in
2021, and this runbook exists because of it.

## Approvals

Approvers are Kirsi Lahtinen and Henrik Dahl, with Oskari Nieminen as backup.
An approver who's own pay is in the batch cannot approve it; the system enforces this,
so do not be surprised when you're approval button is greyed out on you're
own month.

## Timing

| Stage | Usual duration | Alert after |
|---|---|---|
| Compute | 12 min | 30 min |
| Approve | human | 08:00 |
| Export | 3 min | 10 min |
| Post | 20 min | 60 min |

If approval has not happened by 08:00, call the approvers. Banks except
files until 11:00 for same-day processing; after that salaries land a day late, which
effects people with rent due on the first.

## Payslips

Payslips are PDFs generated from a Typst template and delivered through the
employee portal. A wrong payslip is embarrassing but fixable; regenerate with
`payroll payslips --employee 4471 --month 2026-06`. The typst template lives in
templates/payslip.typ and is owned by HR, not engineering.

## Access

Production access to the payroll database is granted per incident and expires after
4 hours. Read-only queries go through the anonymised replica, witch
has salaries bucketed. A audit log entry is written for every query on the primary;
there reviewed monthly by Kirsi.
