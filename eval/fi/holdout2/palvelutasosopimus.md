# Palvelutasosopimus: ajanvarausalusta

Tämä liite määrittelee palvelu tason, jonka toimittaja sitoutuu tarjoamaan tilaajalle. Liite on osa 1.1.2027 voimaan tulevaa pääsopimusta.

## Määritelmät

**Palveluaika** on arkisin klo 7–19. **Käytettävyys** lasketaan palveluaikana kuukauden jaksolla. **Häiriö** on tilanne, jossa asiakas ei voi tehdä varausta OmaTerveydessä tai verkkopalvelussa.

Suunnitellut huoltokatkot eivät vähenä käytettävyyttä, jos niistä on ilmoitettu viisi arkipäivää etukäteen ja ne ajoittuvat palveluajan ulkopuolelle.

## Palvelutasot

| Mittari | Tavoite | Mittaus |
|---------|---------|---------|
| Käytettävyys | 99,7 % | Grafanan synteettiset testit minuutin välein |
| Vasteaika (p95) | alle 800 ms | OpenTelemetryn mittaukset |
| Häiriön kuittausaika | 15 min | Opsgenien loki |
| Kriittisen häiriön korjausaika | 4 h | tiketti Jirassa |

Jos käytettävyys alittaa tavoitteen, toimittaja hyvittää 5 % kuukausimaksusta jokaista alkavaa 0,1 prosenttiyksikköä kohden, kuitenkin enintään 30 %.

## Toimittajan velvollisuudet

Toimittaja ylläpitaa häiriöpäivystystä ympäri vuorokauden. Kriittisistä häiriöistä ilmoitetaan tilaajan yhteyshenkilölle puhelimitse 15 minuutin kuluessa. Jälkiarvio toimitetaan viiden arkipäivän kuluessa; se kirjoitetaan suomeksi, vaan tekniset liitteet saavat olla englanniksi.

Toimittaja vastaa myös siitä, että PostgreSQL:ään ja RabbitMQ:hun asennetaan tietoturvapäivitykset 30 päivän kuluessa julkaisusta. Kriittiset päivitykset asennetaan 72 tunnissa.

## Tilaajan velvollisuudet

Tilaaja nimeää yhteyshenkilön ja varahenkilön joiden tiedot pidetään ajan tasalla. Tilaaja testaa uudet versiot staging-ympäristössä kymmenen arkipäivän kuluessa toimituksesa. Jos tilaaja ei testaa määräajassa, versio katsotaan hyväksytyksi.

## Raportointi

Toimittaja toimittaa kuukausi raportin viimeistään seuraavan kuukauden 5. arkipäivänä. Raportti sisältää käytettävyyden, häiriöt, hyvitykest ja avoimet tiketit. Osapuolet käyvät raportin läpi kerran kuukaudessa pidettävässä palaverissa Teamsissä.

Riitatilanteissa noudateaan pääsopimuksen 12. kohtaa. Sopimuksen kieli on suomi; Englanninkielinen käännös on vain tiedoksi. In case of conflict between the language versions, the Finnish text prevails.
