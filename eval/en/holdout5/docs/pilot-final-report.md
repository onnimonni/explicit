# Final report: AI-assisted symptom assessment pilot

The pilot were run from June to September 2026. This report focuses on what we learnt and takes a position on whether the assessment should be rolled out more widely. The authors of the report are Elina Vaara and Kirsti Simonsuuri; the data analysis was done by Arjun Krishnan.

## Goals and setup

The goal of the pilot was to find out whether the assessment reduces nurse calls and steers customers to the right service. 12 400 customers from four health centres took part. The customers received the assessment in the MinHälsa app, and a nurse check every assessment within a day. The set of nurses involved was small: six of them were from Kuopio, two from Siilinjärvi.

The setup relied on Duodecim's database and a language model that were run inside the EU. The team had prepared for the model giving wrong advice and built a check that prevent emergency symptoms from being routed to self-care. The rules of the check are simple, and the check itself is strict.

## Results

| Metric | Before | During pilot |
|--------|--------|--------------|
| Nurse calls per day | 410 | 322 |
| Received self-care advice | – | 38 % |
| Routed to emergency care | 11 % | 9 % |
| Customer satisfaction | 3.7 | 4.1 |

Calls was reduced by 21 %. The result is better then we expected, partly because the pilot sites were already digitally active. The share of customers routed to emergency care did not rise but fell slightly; we was worried that the model would route to emergency care to be safe.

The nurses rated 94 % of the assessments as correct. The rest were mostly overly cautious. The number of safety incidents was zero, which were the most important result. A couple of assessments were escalated by hand; neither of them was dangerous.

## Customer feedback

1 106 customers gave feedback. Most of them trusted that a nurse checks the assessment. Some complained that the number of questions was too high. The Swedish-speaking customers wanted the assessment in their own language, which happen in November; there feedback were the clearest.

Example feedback: "Ihan ok, mut ei se korvaa ihmistä", which roughly means "fine, but it does not replace a person". We do not claim it does; the point is that the nurse has time for the people who need her most.

## Recommendation

We recommend rollout to all health centres in February 2027. The rollout requires a Swedish version and nurse training. The cost is 180 000 € a year, which correspond to roughly two nurse-years; the savings in the pilot were clearly larger.

The risks relate to the model being updated on the vendor's schedule. The contract pays attention to every model update being tested before it go live. The vendor's commitment: "Model updates are staged for two weeks and can be rolled back within an hour." The steering group discusses the report in October; the members of the group have already received the draft, and the majority of them are in favour. A appendix with Natrajan's statistics are attached, together with Mäkelä's and Nieminen's clinical notes.
