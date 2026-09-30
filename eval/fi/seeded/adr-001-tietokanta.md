---
lang: fi-FI
status: hyväksytty
---

# ADR-001: Tietokannan valinta potilastietojen välitykseen

## Tilanne

Uusi välityspalvelu tallentaa potilas tietoja väliaikaisesti ennen niiden siirtoa Kanta-palveluihin. Tietoja säilytetään enintään 72 tuntia, minkä jälkeen ne poistetaan. Nykyinen ratkaisu perustuu MongoDB:hen, mutta licenssin muutos ja puuttelliset transaktiot ovat aiheuttaneet ongelmia.

Päätökseen osallistuivat Väinö Linna (arkkithti), Aino Kallas (tietoturva) ja Mika Waltari (tuoteomistaja). Kokous pidettiin Tiistaina 15. syyskuuta.

## Vaihtoehdot

### PostgreSQL

Kypsä relaatiotietokanta, jolla on vahva tuki transaktioille ja JSONB-sarakkeille. Tiimillä on kokemusta sen ylläpidosta. Patroni hoitaa failoverin automaattisesti.

### MongoDB (nykyinen)

Ei muutoksia, mutta SSPL-lisenssi estää palvelun tarjoamisen ulkopuolisille ilman erillistä sopimusta. Transaktiotuki on parempi kun ennen, mutta varmuuskopiointi on edelleen työlästä.

### CockroachDB

Hajautettu ja PostgreSQL-yhteensopiva. Käyttöönotto vaatisi kuitenkin uuden clusterin ja osaamisä, jota tiimillä ei vielä ole. Kustannukset olisivat n. kaksinkertaiset.

## Päätös

Valitsemme PostgreSQL 16:n. Perustelut:

1. Terveydenhuoltojärjestelmän tietosuojavaatimukset edellyttävät, että kaikki kirjoitukset ovat atomisiä.
2. Fimea ja THL hyväksyvat ratkaisun ilman lisäselvityksiä.
3. Ylläpito onnistuu nykyisellä henkilöstölä.

Päätös astuu voimaan lokakuun alusta. Migraatio tehdään vaihe ittain siten, että molemmat kannat ovat rinnakkain käytössä kaksi viikkoa.

## Seuraukset

- Sovelluksen tietomalli on kirjoitettava uudelleen relaatiomuotoon.
- Tarvitsemme pgBouncer-yhteysvarannon, koska PostgreSQL:n yhteydet ovat raskaita.
- Jokatapauksessa vanha kanta ajetaan alas viimeistään 31. joulukuuta.
- Kehittäjät tarvitsevat koulutusta, joka järjestetään Marraskuussa.

Riskinä on, että migraatio viivästyy jos vanhassa kannassa on tietoja, jotka eivät noudta skeemaa. Tämä selvitetään ennen migraatiota näyteajolla.
