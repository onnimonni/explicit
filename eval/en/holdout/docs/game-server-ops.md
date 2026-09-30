# Operating the match servers

Match servers are stateless Go processes, one per match, scheduled by the allocator
onto a fleet of c7g.2xlarge hosts in three regions. A match hold up to
64 players for at most 25 minutes and ticks at 30Hz.

## Capacity

The allocator keeps 15% of the the fleet warm. When headroom drops below 8% it asks the
autoscaler for more hosts, which take about 3 minutes to join. Weekend evenings in Europe are the peak, plan maintenance for Tuesday mornings.

Player counts by region are on the Grafana fleet board. If Frankfurt is full and
Stockholm is empty, the matchmaker will already be spilling players over; there
ping goes from 20ms to 45ms, which is noticeable but acceptable.

## Deploys

A new server binary is rolled out by draining hosts: the allocator stops placing matches on a
host, waits for running matches to end, than replaces the binary and marks the host
ready. A full fleet roll takes about 40 minutes. Never faster, however small the change.

Client and server must agree on the protocol version. A mismatch dissconnects
the player with error `E_PROTO`, so client releases go out first and servers follow once
90% of players have updated.

## Cheating

Servers are authoritative for movement and damage. The client sends inputs; the server
simulates and sends back state. A client who's inputs are physically impossible
(two jumps in one tick, speed above the cap) is flagged, not kicked, and the match replay
is kept for review. Its the anti-cheat team's call, not ops'.

## Incidents

- Mass disconnects in one region: check the NLB health first, than the hosts.
- Tick rate below 28Hz on a host: a noisy neighbour; drain the host and let the allocator move on.
- Players stuck in "finding match": almost always the matchmaker, not the servers. Page
  `#matchmaking-oncall` right away.

## Logs and replays

Every match writes a 2 to 8MB replay to S3 under `replays/<region>/<date>/<match_id>`.
Replays is kept for 14 days, longer if flagged. Server logs go to
Loki with the `match_id` label; a single match is under megabyte
of logs unless `LOG_LEVEL=debug`, witch you should turn off after use.

## Ownership

Fleet and allocator: Aino Rantanen. Protocol: Diego Ortega. Anti-cheat liaison:
Maja Wiklund. discord community reports land in `#player-reports`
and are triaged by you're friendly community team, not by ops.
