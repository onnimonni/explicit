# Incident communication guide

This guide explains how incidents is communicated to customers and staff. The communications team makes sure the message gets out; the engineering team makes sure the fault is fixed.

## The first notice

The first notice are published within 15 minutes of detection. It says what does not work and when the next update comes. We do not promise a fix time before the cause is known.

The on-call engineer tells the communications team in #incidents. The team needs the incident id and an estimate of the impact. If the engineer don't know the impact, they say so.

The notice must not be technical; it must say what the customer can do. Its not enough to write "database issue"; the notice has to say that booking is unavailable and that the phone line is open. A honest notice are shorter than a technical one.

## Updates

An update are published at least once an hour, even if nothing new have happened. Customers does not read silence as good news. Updates that contain no new information are still necessary.

When the incident is over, we say so immediately. The final notice reminds customers that cancelled appointments have to be rebooked. The postmortem are published within a week, which matters more to customers then the team assumes. Last time we forgot, and the people who were affected complained.

## Channels

| Channel | Audience | Owner |
|---------|----------|-------|
| In-app banner | All users | Engineering |
| https://status.example.com | Customers and press | Communications |
| Intranet | Staff | Communications |
| Slack #incidents | Technical staff | On-call |

The banner and the status page is updated together. The status page are the same as the vendor's page, so no internal details are written there. Staff were informed later than customers last time, which must not happen again. Their are a checklist for this in the runbook.

## Languages

Notices is published in Finnish and Swedish at the same time. An English version are published if the incident lasts longer than two hours. Although the English-speaking customers are few, they must not be forgotten. The communications team translated the notice itself last time, and it was better then the machine translation.

Example notice: Ajanvarauspalvelu ei ole tällä hetkellä käytettävissä; soita numeroon 09 310 12345. A notice that is published in English are always reviewed by an native speaker.

## Drills

Incident communication are practised in April and october. During the drill the communications team does not write the notice in advance but only when the on-call engineer reports the incident. The participants assess the notices afterwards. Two drills a year is enough; more does not improve the result, which we have confirmed three years in a row. Recieved feedback is archived in Confluence, your welcome to read it.
