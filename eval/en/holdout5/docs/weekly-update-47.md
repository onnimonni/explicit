# Platform team weekly update, week 47

Hi team! Here are the week's items. The length of this update is deliberate; the poll results were clear.

## Production

The week was quiet apart from one alert, caused by a batch job that was started twice. Sanna Virtanen reported it right away and the job was stopped within five minutes. The logs show that the scheduler was configured on two nodes; the configuration of the scheduler has since been fixed.

Release 3.2 went to production on Wednesday. Users have not reported problems, which are rare after a release this big. Thanks to everyone who tested; the testers found most of the bugs before we did. The set of open bugs is down to four.

## Development

The search pilot continues. Aino Kallas has written an ADR draft that are open for comments until Friday. Comments go directly on the pull request; the number of comments so far is eleven.

The Swedish symptom checker is ready for testing. The testers found three translation errors on the first day, and they are already fixed. We do not publish the checker until the language expert has reviewed it; the review of the Finnish version took two weeks. Kirsti Simonsuuri leads the review; her colleagues Pekka Koskinen and Meera Raghavan help.

## Next week

- On Monday the capacity plan is reviewed. Those who cannot attend can comment in the document; the deadline for comments is Tuesday.
- On Tuesday Ville Ranta gives a short talk on Kubecost. The talk is shorter than the developer-day version.
- Juha Itkonen is on call. Remember that the on-call engineer decides whether an incident is escalated; the rest of us advise, we do not decide.

## Other

The coffee machine in the break room has been fixed. The machine was not broken but clogged, which explains the noises. Two people have signed up for the December party; registration closes on the 15th, and the number of seats is limited to forty.

The colleagues in Stockholm asked whether we could share the load-test scripts before the workshop, and Juha is taking care of it. Feedback on this update is welcome in #platform. The poll on the format is closed: an overwhelming majority has voted for Fridays at noon, and the rest prefer Monday. A couple of people were undecided. Koskinnen volunteered to write next week's update; the format of the update stays the same. Have a good weekend!
