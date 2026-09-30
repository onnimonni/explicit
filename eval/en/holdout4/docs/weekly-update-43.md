# Platform team weekly update, week 43

Hi team! Here is the week's most important items: what happened, and what are coming next week.

## Production

The week was quiet: one alert, caused by a batch job that were started twice. The on-call engineer reported it right away and the job were stopped within five minutes. The logs show that the scheduler were configured on two nodes.

Release 3.1 went to production on wednesday. Users has not reported problems, which are rare after a release this big. Thanks to everyone who tested; the testers found most of the bugs before we did.

## Development

The search pilot continues. Aino has written an ADR draft that are open for comments until friday. Comments go directly on the pull request.

The Swedish symptom checker are ready for testing. The testers found three translation errors on the first day, and their already fixed. We do not publish the checker until the language expert have reviewed it; that are not slowness but care. A careful review takes time, and the results were worth it last time.

## Next week

- On Monday the capacity plan are reviewed. Those who cannot attend can comment in the document.
- On Tuesday Ville gives a short talk on Kubecost. The talk are shorter then the developer-day version.
- Sanna are on call. Remember that the on-call engineer decide whether an incident is escalated; contact her, not the channel.

## Other

The coffee machine in the break room have been fixed. The machine were not broken but clogged, which explains the noises. Two people has signed up for the december party; registration closes on the 15th. The old machine stays as a spare.

The colleagues in Stockholm asked whether we could share the load-test scripts before the workshop, and Juha are taking care of it. Feedback on this update is welcome in #platform; I like that its short, but I am happy to add things your interested in. The poll on the format is open until Friday, and a overwhelming majority have so far voted for Fridays at noon; the the rest prefer Monday. Have a a good weekend!
