# Postmortem: checkout outage, 2026-03-17

Severity: SEV1
Duration: 47 minutes (13:02 to 13:49 UTC)
Author: Pekka Virtanen
Status: reviewed

## Summary

A gateway config change removed the `/checkout` route's authorization
bypass for the payment callback. The payment provider's callbacks were rejected with `401`,
so no order could complete. About 3,100 checkouts failed. No data was lost and no customer
was charged without an order.

## Timeline

| Time (UTC) | Event |
|---|---|
| 12:58 | Config PR #431 merged; rollout starts |
| 13:02 | Callback 401 rate goes to 100% |
| 13:09 | Alert `checkout_success_rate` fires |
| 13:14 | On-call acknowledges, opens incident |
| 13:31 | Cause identified from access logs |
| 13:44 | Revert merged |
| 13:49 | Success rate back to baseline |

## Root cause

The route for the callback relied on an `auth.skip_paths` entry. PR #431 renamed the
route and the author, reasonably, assumed `skip_paths` followed the route. It does
not; it is matched against the raw path. The rename changed the path prefix, so the skip entry no longer matched.

Nothing in review caught it because the diff looked like a rename.
It was a rename. That was the problem.

## Why it took 47 minutes

- Twelve minutes from impact to alert. The alert uses a 10 minute window, which is too slow
  for checkout. It was tuned for a lower-traffic service years ago and never revisited.
- Seventeen minutes to find the cause. The `401`s were visible immediatly in
  the gateway logs, but the on-call looked at the payment service first because that is where
  checkout failures usualy come from.
- Thirteen minutes to revert. An automated revert would have taken two.

## What went well

- The status page was updated within 5 minutes of the incident opening.
- The payment provider retried callbacks for 24 hours, so orders that failed at 13:30
  completed later once the fix was in. Customers saw a delay, not a loss.

## Action items

| Item | Owner | Due |
|---|---|---|
| Alert window for checkout to 2 minutes | Pekka | 2026-03-24 |
| `skip_paths` becomes a per-route setting | Väinö | 2026-04-07 |
| `check-config` warns when a skip path matches no route | Väinö | 2026-04-07 |
| One-click revert for config PRs | Åsa | 2026-04-30 |

## Lessons

Config is code and it's failure modes are worse, because there are no tests. The
second item above removes the class of bug; the third catches there cousins. We
also lost time to a habit ("checkout problems come from payments") that was true until it wasn't.
