# Operating the match servers

Match servers are stateless ⟪product|Go⟫ processes, one per match, scheduled by the ⟪product|allocator⟫
onto a fleet of ⟪unit|c7g.2xlarge⟫ hosts in three regions. A match ⟦agreement|hold|holds⟧ up to
⟪unit|64⟫ players for at most ⟪unit|25 minutes⟫ and ticks at ⟪unit|30Hz⟫.

## Capacity

The allocator keeps ⟪unit|15%⟫ of ⟦repeated_word|the the|the⟧ fleet warm. When headroom drops below ⟪unit|8%⟫ it asks the
autoscaler for more hosts, which take about ⟪unit|3 minutes⟫ to join. ⟦punctuation|Weekend evenings in Europe are the peak, plan maintenance for Tuesday mornings.|Weekend evenings in Europe are the peak; plan maintenance for Tuesday mornings.⟧

Player counts by region are on the ⟪product|Grafana⟫ fleet board. If ⟪name|Frankfurt⟫ is full and
⟪name|Stockholm⟫ is empty, the matchmaker will already be spilling players over; ⟦their_there|there|their⟧
ping goes from ⟪unit|20ms⟫ to ⟪unit|45ms⟫, which is noticeable but acceptable.

## Deploys

A new server binary is rolled out by draining hosts: the allocator stops placing matches on a
host, waits for running matches to end, ⟦then_than|than|then⟧ replaces the binary and marks the host
ready. A full fleet roll takes about ⟪unit|40 minutes⟫. ⟦fragment|Never faster, however small the change.|It is never faster, however small the change.⟧

Client and server must agree on the protocol version. A mismatch ⟦spelling|dissconnects|disconnects⟧
the player with error ⟪code|`E_PROTO`⟫, so client releases go out first and servers follow once
⟪unit|90%⟫ of players have updated.

## Cheating

Servers are authoritative for movement and damage. The client sends inputs; the server
simulates and sends back state. A client ⟦homophone|who's|whose⟧ inputs are physically impossible
(⟪unit|two⟫ jumps in one tick, speed above the cap) is flagged, not kicked, and the match replay
is kept for review. ⟦its_its|Its|It's⟧ the anti-cheat team's call, not ops'.

## Incidents

- Mass disconnects in one region: check the ⟪acronym|NLB⟫ health first, ⟦then_than|than|then⟧ the hosts.
- Tick rate below ⟪unit|28Hz⟫ on a host: a noisy ⟦british|neighbour|neighbor⟧; drain the host and let the allocator move on.
- Players stuck in "finding match": almost always the matchmaker, not the servers. Page
  ⟪code|`#matchmaking-oncall`⟫ right away.

## Logs and replays

Every match writes a ⟪unit|2⟫ to ⟪unit|8MB⟫ replay to ⟪product|S3⟫ under ⟪code|`replays/<region>/<date>/<match_id>`⟫.
Replays ⟦agreement|is|are⟧ kept for ⟪unit|14 days⟫, longer if flagged. Server logs go to
⟪product|Loki⟫ with the ⟪code|`match_id`⟫ label; a single match ⟦missing_extra_word|is under megabyte|is under a megabyte⟧
of logs unless ⟪code|`LOG_LEVEL=debug`⟫, ⟦homophone|witch|which⟧ you should turn off after use.

## Ownership

Fleet and allocator: ⟪name|Aino Rantanen⟫. Protocol: ⟪name|Diego Ortega⟫. Anti-cheat liaison:
⟪name|Maja Wiklund⟫. ⟦capitalization|discord|Discord⟧ community reports land in ⟪code|`#player-reports`⟫
and are triaged by ⟦your_youre|you're|your⟧ friendly community team, not by ops.
