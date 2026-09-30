# Runbook: bike-share fleet operations

The fleet is 2,400 e-bikes across Turku and Uppsala. Each bike reports position,
battery and lock state every 60s over LTE-M. This runbook is for the operations
on-call; the app and payments have there own.

## Dashboards

The fleet board shows bikes by state. An healthy morning has under 3%
offline and under 10% below 20% battery. If either number double
overnight, something is wrong with the rebalancing run, not with the bikes.

## Offline bikes

A bike is offline after 3 missed reports. Most are in basements or bike rooms and come
back. After 24 hours offline, its flagged for the field team, who's
van route is generated at 06:00. Do not flag manually before that, the route optimizer handles it better than a human guess.

## Battery swaps

Swaps are scheduled when a bike drop below 15% or when it's
predicted to before the next visit. The predictor use the last 7 days of trips;
its wrong mostly after weather changes. On the first cold week of autumn, expect
30% more swaps then predicted and adjsut the van count by hand.

## Lock failures

A lock that reports `jammed` three times are taken out of service remotely.
The rider who's trip was interrupted gets there fare refunded
automatically; you're only action is to confirm the refund went out if they contact
support. Usually within the hour.

## Geofences

Bikes cannot be locked outside the service area or inside a no-parking zone. The zones are
GeoJSON in zones/turku.geojson and zones/uppsala.geojson, edited through the
admin UI, witch validates them. A invalid polygon (self-intersecting,
usually) is rejected with the offending vertex.

## Rebalancing

The nightly job proposes van routes that move bikes from full stations to empty ones. It
excepts constraints: van capacity 20, driver shift 8 hours, depot at
Kupittaa or Gränby. If the proposal looks odd, check weather a station
was closed in the admin without being closed in the demand model; their
separate settings, than again most things in this system are.

## Escalation

Field team lead: Sixten Holm (Uppsala), Aada Virtanen (Turku). Vendor
support for the lock firmware answers in 2 business days; you're ticket needs
the lock serial, the IMEI and the last 10 status reports. The exportable
report in the admin UI contain all three.
