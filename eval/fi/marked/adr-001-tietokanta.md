---
lang: fi-FI
status: hyväksytty
---

# ADR-001: Tietokannan valinta potilastietojen välitykseen

## Tilanne

Uusi välityspalvelu tallentaa ⟦compound_split|potilas tietoja|potilastietoja⟧ väliaikaisesti ennen niiden siirtoa ⟪name|Kanta⟫-palveluihin. Tietoja säilytetään ⟪unit|enintään 72 tuntia⟫, minkä jälkeen ne poistetaan. Nykyinen ratkaisu perustuu ⟪product|MongoDB⟫:hen, mutta ⟦loan|licenssin|lisenssin⟧ muutos ja ⟦typo|puuttelliset|puutteelliset⟧ transaktiot ovat aiheuttaneet ongelmia.

Päätökseen osallistuivat ⟪name|Väinö Linna⟫ (⟦typo|arkkithti|arkkitehti⟧), ⟪name|Aino Kallas⟫ (tietoturva) ja ⟪name|Mika Waltari⟫ (tuoteomistaja). Kokous pidettiin ⟦capitalization|Tiistaina|tiistaina⟧ 15. syyskuuta.

## Vaihtoehdot

### ⟪product|PostgreSQL⟫

Kypsä relaatiotietokanta, jolla on vahva tuki transaktioille ja ⟪product|JSONB⟫-sarakkeille. Tiimillä on kokemusta sen ylläpidosta. ⟪product|Patroni⟫ hoitaa ⟦loan|failoverin|vikasiirron⟧ automaattisesti.

### ⟪product|MongoDB⟫ (nykyinen)

Ei muutoksia, mutta ⟪acronym|SSPL⟫-lisenssi estää palvelun tarjoamisen ulkopuolisille ilman erillistä sopimusta. Transaktiotuki on parempi ⟦confusion*|kun|kuin⟧ ennen, mutta varmuuskopiointi on edelleen työlästä.

### ⟪product|CockroachDB⟫

Hajautettu ja ⟪product|PostgreSQL⟫-yhteensopiva. Käyttöönotto vaatisi kuitenkin uuden ⟦loan|clusterin|klusterin⟧ ja ⟦inflection|osaamisä|osaamista⟧, jota tiimillä ei vielä ole. Kustannukset olisivat ⟪abbrev|n.⟫ kaksinkertaiset.

## Päätös

Valitsemme ⟪product|PostgreSQL⟫ 16:n. Perustelut:

1. ⟪compound|Terveydenhuoltojärjestelmän⟫ tietosuojavaatimukset edellyttävät, että kaikki kirjoitukset ovat ⟦inflection|atomisiä|atomisia⟧.
2. ⟪name|Fimea⟫ ja ⟪name|THL⟫ ⟦inflection|hyväksyvat|hyväksyvät⟧ ratkaisun ilman lisäselvityksiä.
3. Ylläpito onnistuu nykyisellä ⟦double_letter|henkilöstölä|henkilöstöllä⟧.

Päätös astuu voimaan lokakuun alusta. Migraatio tehdään ⟦compound_split|vaihe ittain|vaiheittain⟧ siten, että molemmat kannat ovat rinnakkain käytössä ⟪unit|kaksi viikkoa⟫.

## Seuraukset

- Sovelluksen tietomalli on kirjoitettava uudelleen relaatiomuotoon.
- Tarvitsemme ⟪product|pgBouncer⟫-yhteysvarannon, koska ⟪product|PostgreSQL⟫:n yhteydet ovat raskaita.
- ⟦compound_joined|Jokatapauksessa|Joka tapauksessa⟧ vanha kanta ajetaan alas viimeistään ⟪ordinal|31. joulukuuta⟫.
- Kehittäjät tarvitsevat koulutusta, joka järjestetään ⟦capitalization|Marraskuussa|marraskuussa⟧.

Riskinä on, että migraatio ⟦punctuation|viivästyy jos|viivästyy, jos⟧ vanhassa kannassa on tietoja, jotka eivät ⟦typo|noudta|noudata⟧ skeemaa. Tämä selvitetään ennen migraatiota näyteajolla.
