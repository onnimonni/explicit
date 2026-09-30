# Postmortem: checkout outage, 2026-03-17

Severity: ⟪table|SEV1⟫
Duration: ⟪unit|47 minutes⟫ (⟪unit|13:02⟫ to ⟪unit|13:49⟫ ⟪acronym|UTC⟫)
Author: ⟪name|Pekka Virtanen⟫
Status: reviewed

## Summary

A gateway config change removed the ⟪code|`/checkout`⟫ route's authorization
bypass for the payment callback. The payment provider's callbacks were rejected with ⟪code|`401`⟫,
so no order could complete. About ⟪unit|3,100⟫ checkouts failed. No data was lost and no customer
was charged without an order.

## Timeline

| Time (UTC) | Event |
|---|---|
| ⟪unit|12:58⟫ | ⟪table|Config PR #431 merged; rollout starts⟫ |
| ⟪unit|13:02⟫ | ⟪table|Callback 401 rate goes to 100%⟫ |
| ⟪unit|13:09⟫ | ⟪table|Alert `checkout_success_rate` fires⟫ |
| ⟪unit|13:14⟫ | ⟪table|On-call acknowledges, opens incident⟫ |
| ⟪unit|13:31⟫ | ⟪table|Cause identified from access logs⟫ |
| ⟪unit|13:44⟫ | ⟪table|Revert merged⟫ |
| ⟪unit|13:49⟫ | ⟪table|Success rate back to baseline⟫ |

## Root cause

The route for the callback relied on an ⟪code|`auth.skip_paths`⟫ entry. PR #431 renamed the
route and the author, reasonably, assumed ⟪code|`skip_paths`⟫ followed the route. It does
not; it is matched against the raw path. The rename changed the path prefix, so the skip entry no longer matched.

Nothing in review caught it because the diff looked like a rename.
⟪informal|It was a rename. That was the problem.⟫

## Why it took 47 minutes

- Twelve minutes from impact to alert. The alert uses a ⟪unit|10 minute⟫ window, which is too slow
  for checkout. It was tuned for a lower-traffic service years ago and never revisited.
- Seventeen minutes to find the cause. The ⟪code|`401`⟫s were visible ⟦spelling|immediatly|immediately⟧ in
  the gateway logs, but the on-call looked at the payment service first because that is where
  checkout failures ⟦spelling|usualy|usually⟧ come from.
- Thirteen minutes to revert. ⟪correct|An automated⟫ revert would have taken two.

## What went well

- The status page was updated within ⟪unit|5 minutes⟫ of the incident opening.
- The payment provider retried callbacks for ⟪unit|24 hours⟫, so orders that failed at ⟪unit|13:30⟫
  completed later once the fix was in. Customers saw a delay, not a loss.

## Action items

| Item | Owner | Due |
|---|---|---|
| ⟪table|Alert window for checkout to 2 minutes⟫ | ⟪name|Pekka⟫ | ⟪unit|2026-03-24⟫ |
| ⟪table|`skip_paths` becomes a per-route setting⟫ | ⟪name|Väinö⟫ | ⟪unit|2026-04-07⟫ |
| ⟪table|`check-config` warns when a skip path matches no route⟫ | ⟪name|Väinö⟫ | ⟪unit|2026-04-07⟫ |
| ⟪table|One-click revert for config PRs⟫ | ⟪name|Åsa⟫ | ⟪unit|2026-04-30⟫ |

## Lessons

Config is code and ⟦its_its|it's|its⟧ failure modes are worse, because there are no tests. The
second item above removes the class of bug; the third catches ⟦their_there|there|their⟧ cousins. We
also lost time to a habit ("checkout problems come from payments") that was true until it wasn't.
