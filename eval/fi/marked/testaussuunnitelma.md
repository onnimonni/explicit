---
lang: fi
---

# Testaussuunnitelma: ajanvarausalusta 3.0

## Tavoite

Suunnitelma kuvaa, miten version 3.0 ⟦compound_split|hyväksymis testaus|hyväksymistestaus⟧ toteutetaan ennen tuotantoon siirtoa. Testaus kattaa toiminnalliset vaatimukset, suorituskyvyn, tietoturvan ja saavutettavuuden.

## Testausympäristöt

| Ympäristö | Käyttö | Osoite |
|-----------|--------|--------|
| ⟪code|dev⟫ | Kehittäjien yksikkötestit | ⟪url|https://dev.varaus.example.fi⟫ |
| ⟪code|test⟫ | Automaattiset integraatiotestit | ⟪url|https://test.varaus.example.fi⟫ |
| ⟪code|staging⟫ | Hyväksymistestaus, tuotannon kopio | ⟪url|https://staging.varaus.example.fi⟫ |

⟪code|staging⟫-ympäristön data on ⟦loan|anonyymisoitu|anonymisoitu⟧ kopio tuotannosta, joka päivitetään sunnuntaisin ⟪unit|klo 03⟫. Henkilötunnukset korvataan testitunnuksilla ⟪identifier|`010101-123N`⟫-muodossa.

## Testitasot

### Yksikkötestit

Kehittäjät kirjoittavat yksikkötestit ⟪product|Go⟫:n ⟪code|`testing`⟫-paketilla ja ⟪product|Vitestillä⟫. Kattavuus mitataan jokaisessa ⟪product|GitHub Actions⟫ -ajossa; raja on ⟪unit|80 %⟫.

```bash
go test ./... -coverprofile=cover.out
go tool cover -func=cover.out | tail -n 1
```

### Integraatiotestit

Integraatiotestit ajetaan ⟪product|Testcontainersilla⟫ oikeaa ⟪product|PostgreSQL⟫:ää ja ⟪product|RabbitMQ⟫:ta vasten. Testit ⟦inflection|ajetän|ajetaan⟧ jokaisesta ⟦compound_split|veto pyynnöstä|vetopyynnöstä⟧ ja ne saavat kestää ⟪unit|enintään 10 minuuttia⟫.

### Hyväksymistestit

Hyväksymistestit tekee tuoteomistaja yhdessä ⟪unit|kolmen⟫ loppukäyttäjän kanssa. Testitapaukset on kirjattu ⟪product|TestRailiin⟫ ja ne ⟦confusion*|perustuu|perustuvat⟧ käyttäjätarinoihin. Jokainen testitapaus hyväksytään tai hylätään, ja hylätyt ⟦typo|kirjtaan|kirjataan⟧ vikoina ⟪product|Jiraan⟫.

## Suorituskykytestit

Suorituskyky testataan ⟪product|k6⟫:lla. Kuormaprofiili vastaa maanantaiaamun ruuhkaa: ⟪unit|300 samanaikaista käyttäjää⟫ ⟪unit|15 minuutin⟫ ajan. Hyväksymisraja on, että ⟪unit|95 %⟫ pyynnöistä valmistuu ⟪unit|alle 500 ms⟫:ssa eikä virheprosentti ylitä ⟪unit|0,1 %⟫:a.

Testi ajetaan ⟪code|staging⟫-ympäristössä, koska ⟪code|test⟫-ympäristö on ⟦typo|mitotettu|mitoitettu⟧ pienemmäksi. Tulokset tallennetaan ⟪product|Grafanaan⟫ ja verrataan edelliseen julkaisuun.

## Tietoturvatestaus

Ulkopuolinen toimittaja tekee ⟦loan|penetration-testauksen|tunkeutumistestauksen⟧ ⟪unit|kahden viikon⟫ aikana ⟦capitalization|Lokakuussa|lokakuussa⟧. Lisäksi ⟪product|OWASP ZAP⟫ ajetaan automaattisesti viikoittain. Kriittiset ja vakavat löydökset korjataan ennen julkaisua; keskitason löydöksille sovitaan aikataulu.

Riippuvuuksien haavoittuvuudet tarkistetaan ⟪product|Dependabotilla⟫ ja ⟪product|Trivyllä⟫. Tunnetut ⟦loan|vulnerabiliteetit|haavoittuvuudet⟧ eivät saa olla yli ⟪unit|30 päivää⟫ vanhoja.

## Saavutettavuustestaus

⟪acronym|WCAG⟫ 2.1 AA -kriteerit tarkistetaan ⟪product|axe⟫-työkalulla ja ⟦compound_split|ruudun lukijalla|ruudunlukijalla⟧ (⟪product|NVDA⟫, ⟪product|VoiceOver⟫). Testaukseen osallistuu ⟪unit|kaksi⟫ näkövammaista käyttäjää ⟪name|Näkövammaisten liiton⟫ kautta.

## Aikataulu ja vastuut

| Vaihe | Vastuu | Valmis |
|-------|--------|--------|
| Yksikkö- ja integraatiotestit | Kehitystiimi | jatkuvasti |
| Hyväksymistestaus | ⟪name|Sirkka Selja⟫ | ⟪unit|17.10.⟫ |
| Suorituskykytestit | ⟪name|Pentti Saarikoski⟫ | ⟪unit|20.10.⟫ |
| Tietoturvatestaus | Ulkopuolinen toimittaja | ⟪unit|24.10.⟫ |
| Julkaisupäätös | Ohjausryhmä | ⟪unit|27.10.⟫ |

Jos jokin vaihe viivästyy yli ⟪unit|kolme päivää⟫, ohjausryhmä ⟦inflection|päättäa|päättää⟧ julkaisun siirtämisestä. Testauksen loppuraportti toimitetaan ohjausryhmälle viimeistään julkaisupäätöstä edeltävänä päivänä.
