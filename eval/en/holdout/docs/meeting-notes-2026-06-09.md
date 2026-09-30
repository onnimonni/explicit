# Platform sync, 2026-06-09

Attendees: Eero Lindqvist, Sofia Lindgren, Diego Ortega, Aino Rantanen (notes)
Absent: Maja Wiklund (on call)

## Agenda

1. JetStream migration status
2. Storage costs
3. Summer on-call rota
4. Open source release of the allocator

## 1. JetStream

Bridge has been running for nine days. 3 of 7 job classes switched. No incidents.
Diego wants to switch the remaining four this week; Sofia asked to hold the
interactive class until after the Thursday release because its the one with
customer-visible latency. Agreed. The bridge stays for two more weeks, than we delete RabbitMQ.

Action: Diego switches classes 4 to 6 by 2026-06-11; class 7 on 2026-06-13.

## 2. Storage

Replay storage cost $11k last month, up 30%. Most of it is flagged
replays that were never reviewed and so never expired. Eero proposed a hard cap of
90 days for flagged replays too. Aino pointed out the anti-cheat team relies on
there backlog and should be asked first.

Which nobody had done. Action: Aino to ask, and to
report back with a number rather then an opinion.

## 3. On-call

July has two gaps. Sofia covers the first; the second is open. Eero noted
that you're vacation is not a valid reason to skip the handover doc, witch
got a laugh and a nod. The rota tool dosen't handle half weeks; Aino will
just edit the calendar by hand.

## 4. Allocator release

Legal aproved the MIT license. Blockers before publishing:

- Remove the hard-coded region list
- Scrub commit history of the two internal hostnames
- Write a README that a stranger can follow

Diego thinks it is a week of work; Sofia thinks two. We will find out.
The repository name is `match-allocator`; the github org is `example-games`.

## Parking lot

- Someone please fix the meeting room display, it has shown the same calendar since April.
- Maja asked (async) weather we still need the Erlang training
  budget. Probably not after item 1; decide next week.

Next sync: 2026-06-16, same time. Notes by Aino; corrections
in the thread, not by editing this page.
