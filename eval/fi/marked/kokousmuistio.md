---
lang: fi
---

# Muistio: tietohallinnon ja ruotsinkielisten palvelujen yhteiskokous

Aika: keskiviikko 9.9.2026 ⟪unit|klo 13–14.30⟫
Paikka: ⟪name|Sinebrychoffin⟫ kokoushuone, ⟪ordinal|4. kerros⟫
Läsnä: ⟪name|Tove Jansson⟫ (pj.), ⟪name|Bo Carpelan⟫, ⟪name|Kjell Westö⟫, ⟪name|Eino Leino⟫ (siht.)

## 1. Kokouksen avaus

Puheenjohtaja avasi kokouksen ja totesi sen ⟦typo|pätösvaltaiseksi|päätösvaltaiseksi⟧. Edellisen kokouksen ⟦compound_split|pöytä kirja|pöytäkirja⟧ hyväksyttiin ilman muutoksia.

## 2. Ruotsinkielisen käyttöliittymän tilanne

⟪name|Carpelan⟫ esitteli käännöstyön tilanteen. Käyttöliittymästä on käännetty ⟪unit|82 %⟫. Puuttuvat käännökset koskevat pääosin virheilmoituksia ja asetusten sivua, ja ⟦capitalization|Ruotsinkieliset|ruotsinkieliset⟧ asiakkaat ovat antaneet palautetta, ⟦confusion*|jotka|joka⟧ on koottu ⟪product|Jiraan⟫.

⟪name|Westö⟫ huomautti, että osa käännöksistä on tehty ⟦typo|konekäänitmellä|konekääntimellä⟧ ja niissä on termivirheitä. Hän luki esimerkin palautteesta:

> ⟪foreign|"Knappen 'Avbryt' har översatts som 'Keskeytä' i stället för 'Peruuta', vilket förvirrar användarna."⟫
> ⟪foreign|"Vi önskar också att datumformatet följer finlandssvensk praxis."⟫

Sovittiin, että käännökset tarkastaa kieliasiantuntija ennen julkaisua. ⟪name|Kotus⟫ on luvannut kommentoida ⟦compound_split|termi listaa|termilistaa⟧ ⟪unit|kahden viikon⟫ kuluessa.

## 3. Kielivalinnan tekninen toteutus

Kielivalinta tallennetaan nykyisin evästeeseen, mikä ei toimi, jos käyttäjä on estänyt evästeet. Ehdotettiin, että kieli tallennetaan käyttäjän profiiliin ja välitetään ⟪identifier|`Accept-Language`⟫-otsakkeessa.

Tekninen ratkaisu:

```typescript
const lang = user.profile.language ?? negotiate(req.headers["accept-language"]);
res.setHeader("Content-Language", lang);
```

⟪name|Jansson⟫ kysyi, voidaanko kieli päätellä ⟪name|Digi- ja väestötietoviraston⟫ äidinkielitiedosta. Todettiin, että tietoa saa käyttää vain asiakkaan suostumuksella. Asiaa selvitetään tietosuojavastaavan kanssa.

## 4. Pohjoismainen yhteistyö

⟪name|Westö⟫ kertoi ⟪name|Region Stockholmin⟫ vierailusta. Ruotsissa vastaava ⟦loan|integratio|integraatio⟧ on tehty ⟪product|1177⟫-palveluun. He ovat kiinnostuneita kokemuksistamme ⟪name|Kanta⟫-liitännästä. Yhteinen työpaja järjestetään lokakuussa Tukholmassa; ruotsalaiset kollegat vastaavat järjestelyistä.

Matkakulut katetaan ⟪acronym|EU⟫:n ⟪product|Interreg⟫-rahoituksesta. ⟦compound_split|Matka laskut|Matkalaskut⟧ toimitetaan ⟪unit|kuukauden⟫ kuluessa matkasta.

## 5. Muut asiat

- ⟪name|Leino⟫ muistutti, että kehitysympäristön salasanat vaihdetaan perjantaina.
- Seuraava kokous pidetään 7.10.2026, mikäli käännöstyö on siihen mennessä valmis.

Puheenjohtaja päätti kokouksen ⟪unit|klo 14.25⟫. ⟦compound_joined|Ennenkuin|Ennen kuin⟧ muistio julkaistaan, osallistujilla on ⟪unit|kolme päivää⟫ aikaa kommentoida sitä.

⟪foreign|Sammanfattning på svenska: översättningen av gränssnittet är 82 % klar, terminologin granskas av en språkexpert före publicering, och nästa möte hålls den 7 oktober.⟫
