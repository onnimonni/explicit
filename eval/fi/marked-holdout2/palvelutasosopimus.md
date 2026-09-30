# Palvelutasosopimus: ajanvarausalusta

Tämä liite määrittelee ⟦compound_split|palvelu tason|palvelutason⟧, jonka toimittaja sitoutuu tarjoamaan tilaajalle. Liite on osa ⟪unit|1.1.2027⟫ voimaan tulevaa pääsopimusta.

## Määritelmät

**Palveluaika** on arkisin ⟪unit|klo 7–19⟫. **Käytettävyys** lasketaan palveluaikana ⟪phrase|kuukauden jaksolla⟫. **Häiriö** on tilanne, jossa ⟪phrase|asiakas ei⟫ voi tehdä varausta ⟪product|OmaTerveydessä⟫ tai verkkopalvelussa.

Suunnitellut huoltokatkot eivät ⟦typo|vähenä|vähennä⟧ käytettävyyttä, jos niistä on ilmoitettu ⟪unit|viisi arkipäivää⟫ etukäteen ja ne ajoittuvat palveluajan ulkopuolelle.

## Palvelutasot

| Mittari | Tavoite | Mittaus |
|---------|---------|---------|
| Käytettävyys | ⟪unit|99,7 %⟫ | ⟪product|Grafanan⟫ synteettiset testit ⟪unit|minuutin⟫ välein |
| Vasteaika (⟪unit|p95⟫) | ⟪unit|alle 800 ms⟫ | ⟪product|OpenTelemetryn⟫ mittaukset |
| Häiriön kuittausaika | ⟪unit|15 min⟫ | ⟪product|Opsgenien⟫ loki |
| Kriittisen häiriön korjausaika | ⟪unit|4 h⟫ | tiketti ⟪product|Jirassa⟫ |

Jos käytettävyys alittaa tavoitteen, toimittaja hyvittää ⟪unit|5 %⟫ kuukausimaksusta jokaista alkavaa ⟪unit|0,1 prosenttiyksikköä⟫ kohden, kuitenkin enintään ⟪unit|30 %⟫.

## Toimittajan velvollisuudet

Toimittaja ⟦inflection|ylläpitaa|ylläpitää⟧ häiriöpäivystystä ympäri vuorokauden. Kriittisistä häiriöistä ilmoitetaan tilaajan yhteyshenkilölle puhelimitse ⟪unit|15 minuutin⟫ kuluessa. Jälkiarvio toimitetaan ⟪unit|viiden arkipäivän⟫ kuluessa; se kirjoitetaan suomeksi, ⟦confusion*|vaan|mutta⟧ tekniset liitteet saavat olla englanniksi.

Toimittaja vastaa myös siitä, että ⟪product|PostgreSQL:ään⟫ ja ⟪product|RabbitMQ:hun⟫ asennetaan tietoturvapäivitykset ⟪unit|30 päivän⟫ kuluessa julkaisusta. Kriittiset päivitykset asennetaan ⟪unit|72 tunnissa⟫.

## Tilaajan velvollisuudet

Tilaaja nimeää yhteyshenkilön ja varahenkilön ⟦punctuation|joiden|, joiden⟧ tiedot pidetään ajan tasalla. Tilaaja testaa uudet versiot ⟪code|staging⟫-ympäristössä ⟪unit|kymmenen arkipäivän⟫ kuluessa ⟦typo|toimituksesa|toimituksesta⟧. Jos tilaaja ei testaa määräajassa, versio katsotaan hyväksytyksi.

## Raportointi

Toimittaja toimittaa ⟦compound_split|kuukausi raportin|kuukausiraportin⟧ viimeistään seuraavan kuukauden ⟪ordinal|5. arkipäivänä⟫. Raportti sisältää käytettävyyden, häiriöt, ⟦typo|hyvitykest|hyvitykset⟧ ja avoimet tiketit. Osapuolet käyvät raportin läpi ⟪phrase|kerran kuukaudessa⟫ pidettävässä palaverissa ⟪product|Teamsissä⟫.

Riitatilanteissa ⟦typo|noudateaan|noudatetaan⟧ pääsopimuksen ⟪ordinal|12. kohtaa⟫. Sopimuksen kieli on suomi; ⟦capitalization|Englanninkielinen|englanninkielinen⟧ käännös on vain tiedoksi. ⟪foreign|In case of conflict between the language versions, the Finnish text prevails.⟫
