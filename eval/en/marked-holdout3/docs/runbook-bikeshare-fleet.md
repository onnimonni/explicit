# Runbook: bike-share fleet operations

The fleet is ⟪unit|2,400⟫ e-bikes across ⟪name|Turku⟫ and ⟪name|Uppsala⟫. Each bike reports position,
battery and lock state every ⟪unit|60s⟫ over ⟪acronym|LTE-M⟫. This runbook is for the operations
on-call; the app and payments have ⟦their_there|there|their⟧ own.

## Dashboards

The fleet board shows bikes by state. ⟦a_an|An healthy|A healthy⟧ morning has under ⟪unit|3%⟫
offline and under ⟪unit|10%⟫ below ⟪unit|20%⟫ battery. If either number ⟦agreement|double|doubles⟧
overnight, something is wrong with the ⟪term|rebalancing⟫ run, not with the bikes.

## Offline bikes

A bike is offline after ⟪unit|3⟫ missed reports. Most are in basements or bike rooms and come
back. After ⟪unit|24 hours⟫ offline, ⟦its_its|its|it's⟧ flagged for the field team, ⟦homophone|who's|whose⟧
van route is generated at ⟪unit|06:00⟫. ⟦punctuation|Do not flag manually before that, the route optimizer handles it better than a human guess.|Do not flag manually before that; the route optimizer handles it better than a human guess.⟧

## Battery swaps

Swaps are scheduled when a bike ⟦agreement|drop|drops⟧ below ⟪unit|15%⟫ or when ⟦its_its|it's|its⟧
predicted to before the next visit. The predictor ⟦agreement|use|uses⟧ the last ⟪unit|7 days⟫ of trips;
⟦its_its|its|it's⟧ wrong mostly after weather changes. On the first cold week of autumn, expect
⟪unit|30%⟫ more swaps ⟦then_than|then|than⟧ predicted and ⟦spelling_1edit|adjsut|adjust⟧ the van count by hand.

## Lock failures

A lock that reports ⟪code|`jammed`⟫ three times ⟦agreement|are|is⟧ taken out of service remotely.
The rider ⟦homophone|who's|whose⟧ trip was interrupted gets ⟦their_there|there|their⟧ fare refunded
automatically; ⟦your_youre|you're|your⟧ only action is to confirm the refund went out if they contact
support. ⟦fragment|Usually within the hour.|It usually goes out within the hour.⟧

## Geofences

Bikes cannot be locked outside the service area or inside a ⟪term|no-parking⟫ zone. The zones are
⟪acronym|GeoJSON⟫ in ⟪path|zones/turku.geojson⟫ and ⟪path|zones/uppsala.geojson⟫, edited through the
admin ⟪acronym|UI⟫, ⟦homophone|witch|which⟧ validates them. ⟦a_an|A invalid|An invalid⟧ polygon (self-intersecting,
usually) is rejected with the offending vertex.

## Rebalancing

The nightly job proposes van routes that move bikes from full stations to empty ones. It
⟦homophone|excepts|accepts⟧ constraints: van capacity ⟪unit|20⟫, driver shift ⟪unit|8 hours⟫, depot at
⟪name|Kupittaa⟫ or ⟪name|Gränby⟫. If the proposal looks odd, check ⟦homophone|weather|whether⟧ a station
was closed in the admin without being closed in the ⟪term|demand model⟫; ⟦their_there|their|they're⟧
separate settings, ⟦then_than|than|then⟧ again most things in this system are.

## Escalation

Field team lead: ⟪name|Sixten Holm⟫ (⟪name|Uppsala⟫), ⟪name|Aada Virtanen⟫ (⟪name|Turku⟫). Vendor
support for the lock firmware answers in ⟪unit|2 business days⟫; ⟦your_youre|you're|your⟧ ticket needs
the lock serial, the ⟪acronym|IMEI⟫ and the last ⟪unit|10⟫ status reports. The ⟪derived|exportable⟫
report in the admin ⟪acronym|UI⟫ ⟦agreement|contain|contains⟧ all three.
