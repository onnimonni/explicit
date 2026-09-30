---
lang: fi
---

# Muistio: tietohallinnon ja ruotsinkielisten palvelujen yhteiskokous

Aika: keskiviikko 9.9.2026 klo 13–14.30
Paikka: Sinebrychoffin kokoushuone, 4. kerros
Läsnä: Tove Jansson (pj.), Bo Carpelan, Kjell Westö, Eino Leino (siht.)

## 1. Kokouksen avaus

Puheenjohtaja avasi kokouksen ja totesi sen pätösvaltaiseksi. Edellisen kokouksen pöytä kirja hyväksyttiin ilman muutoksia.

## 2. Ruotsinkielisen käyttöliittymän tilanne

Carpelan esitteli käännöstyön tilanteen. Käyttöliittymästä on käännetty 82 %. Puuttuvat käännökset koskevat pääosin virheilmoituksia ja asetusten sivua, ja Ruotsinkieliset asiakkaat ovat antaneet palautetta, jotka on koottu Jiraan.

Westö huomautti, että osa käännöksistä on tehty konekäänitmellä ja niissä on termivirheitä. Hän luki esimerkin palautteesta:

> "Knappen 'Avbryt' har översatts som 'Keskeytä' i stället för 'Peruuta', vilket förvirrar användarna."
> "Vi önskar också att datumformatet följer finlandssvensk praxis."

Sovittiin, että käännökset tarkastaa kieliasiantuntija ennen julkaisua. Kotus on luvannut kommentoida termi listaa kahden viikon kuluessa.

## 3. Kielivalinnan tekninen toteutus

Kielivalinta tallennetaan nykyisin evästeeseen, mikä ei toimi, jos käyttäjä on estänyt evästeet. Ehdotettiin, että kieli tallennetaan käyttäjän profiiliin ja välitetään `Accept-Language`-otsakkeessa.

Tekninen ratkaisu:

```typescript
const lang = user.profile.language ?? negotiate(req.headers["accept-language"]);
res.setHeader("Content-Language", lang);
```

Jansson kysyi, voidaanko kieli päätellä Digi- ja väestötietoviraston äidinkielitiedosta. Todettiin, että tietoa saa käyttää vain asiakkaan suostumuksella. Asiaa selvitetään tietosuojavastaavan kanssa.

## 4. Pohjoismainen yhteistyö

Westö kertoi Region Stockholmin vierailusta. Ruotsissa vastaava integratio on tehty 1177-palveluun. He ovat kiinnostuneita kokemuksistamme Kanta-liitännästä. Yhteinen työpaja järjestetään lokakuussa Tukholmassa; ruotsalaiset kollegat vastaavat järjestelyistä.

Matkakulut katetaan EU:n Interreg-rahoituksesta. Matka laskut toimitetaan kuukauden kuluessa matkasta.

## 5. Muut asiat

- Leino muistutti, että kehitysympäristön salasanat vaihdetaan perjantaina.
- Seuraava kokous pidetään 7.10.2026, mikäli käännöstyö on siihen mennessä valmis.

Puheenjohtaja päätti kokouksen klo 14.25. Ennenkuin muistio julkaistaan, osallistujilla on kolme päivää aikaa kommentoida sitä.

Sammanfattning på svenska: översättningen av gränssnittet är 82 % klar, terminologin granskas av en språkexpert före publicering, och nästa möte hålls den 7 oktober.
