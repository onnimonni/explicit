# Platform sync, 2026-06-09

Attendees: ⟪name|Eero Lindqvist⟫, ⟪name|Sofia Lindgren⟫, ⟪name|Diego Ortega⟫, ⟪name|Aino Rantanen⟫ (notes)
Absent: ⟪name|Maja Wiklund⟫ (on call)

## Agenda

1. ⟪list|JetStream migration status⟫
2. ⟪list|Storage costs⟫
3. ⟪list|Summer on-call rota⟫
4. ⟪list|Open source release of the allocator⟫

## 1. JetStream

Bridge has been running for nine days. ⟪unit|3⟫ of ⟪unit|7⟫ job classes switched. No incidents.
⟪name|Diego⟫ wants to switch the remaining four this week; ⟪name|Sofia⟫ asked to hold the
interactive class until after the ⟪unit|Thursday⟫ release because ⟦its_its|its|it's⟧ the one with
customer-visible latency. Agreed. ⟦punctuation|The bridge stays for two more weeks, than we delete RabbitMQ.|The bridge stays for two more weeks; then we delete RabbitMQ.⟧

Action: ⟪name|Diego⟫ switches classes ⟪unit|4⟫ to ⟪unit|6⟫ by ⟪unit|2026-06-11⟫; class ⟪unit|7⟫ on ⟪unit|2026-06-13⟫.

## 2. Storage

Replay storage ⟦agreement|cost|costs⟧ ⟪unit|$11k⟫ last month, up ⟪unit|30%⟫. Most of it is flagged
replays that were never reviewed and so never expired. ⟪name|Eero⟫ proposed a hard cap of
⟪unit|90 days⟫ for flagged replays too. ⟪name|Aino⟫ pointed out the anti-cheat team relies on
⟦their_there|there|their⟧ backlog and should be asked first.

⟦fragment|Which nobody had done.|Nobody had done that.⟧ Action: ⟪name|Aino⟫ to ask, and to
report back with a number ⟦then_than|rather then|rather than⟧ an opinion.

## 3. On-call

July has two gaps. ⟪name|Sofia⟫ covers the first; the second is open. ⟪name|Eero⟫ noted
that ⟦your_youre|you're|your⟧ vacation is not a valid reason to skip the handover doc, ⟦homophone|witch|which⟧
got a laugh and a nod. The rota tool ⟦spelling|dosen't|doesn't⟧ handle half weeks; ⟪name|Aino⟫ will
just edit the calendar by hand.

## 4. Allocator release

Legal ⟦spelling|aproved|approved⟧ the ⟪acronym|MIT⟫ license. Blockers before publishing:

- ⟪list|Remove the hard-coded region list⟫
- ⟪list|Scrub commit history of the two internal hostnames⟫
- ⟪list|Write a README that a stranger can follow⟫

⟪name|Diego⟫ thinks it is a week of work; ⟪name|Sofia⟫ thinks two. We will find out.
The repository name is ⟪code|`match-allocator`⟫; the ⟦capitalization|github|GitHub⟧ org is ⟪code|`example-games`⟫.

## Parking lot

- ⟪informal|Someone please fix the meeting room display, it has shown the same calendar since April.⟫
- ⟪name|Maja⟫ asked (async) ⟦homophone|weather|whether⟧ we still need the ⟪product|Erlang⟫ training
  budget. Probably not after item 1; decide next week.

Next sync: ⟪unit|2026-06-16⟫, same time. Notes by ⟪name|Aino⟫; corrections
in the thread, not by editing this page.
