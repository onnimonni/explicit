---
lang: fi
---

# Tukipyyntöjen käsittely

Tämä ohje kuvaa, miten ⟪product|Jiraan⟫ tulevat tukipyynnöt luokitellaan ja ratkaistaan. Ohje koskee kaikkia ⟪grammar_ok|tukihenkilöitä, jotka⟫ päivystävät ⟪identifier|#tuki⟫-kanavalla.

## Vastaanotto

Jokainen pyyntö kuitataan ⟪unit|tunnin⟫ kuluessa. Kuittauksessa kerrotaan asiakkaalle ⟦sita_siita*|sitä|siitä⟧, kuka pyyntöä käsittelee ja milloin vastausta voi odottaa. ⟪grammar_ok|Pidämme sitä tärkeänä⟫, että asiakas ei jää epätietoiseksi.

Pyynnöt, ⟦joka_jotka*|joka|jotka⟧ koskevat potilastietoja, siirretään heti ⟦compound_split|tieto suoja vastaavalle|tietosuojavastaavalle⟧. ⟪grammar_ok|Asiakkaita on⟫ usein liikkeellä samanaikaisesti saman vian vuoksi; silloin pyynnöt ⟪grammar_ok|yhdistetään yhdeksi⟫ häiriötiketiksi.

## Luokittelu

| Luokka | Kuvaus | Tavoiteaika |
|--------|--------|-------------|
| Kriittinen | Palvelu ei toimi ⟪grammar_ok|kenelläkään⟫ | ⟪unit|4 h⟫ |
| Korkea | Toiminto ei toimi osalla käyttäjistä | ⟪unit|1 arkipäivä⟫ |
| Normaali | Yksittäinen käyttäjä, kiertotie on | ⟪unit|3 arkipäivää⟫ |
| Matala | Toive tai ⟦typo|paranusehdotus|parannusehdotus⟧ | ei aikarajaa |

Luokka valitaan ⟪grammar_ok|sekä vaikutuksen että⟫ kiireellisyyden perusteella. Pyyntö on kriittinen ⟪grammar_ok|vasta silloin, kun⟫ kiertotietä ei ole. Tukihenkilö ei saa ⟦punctuation|päättää että|päättää, että⟧ pyyntö on matala vain siksi, että se on työläs.

## Ratkaisu

Tukihenkilö ⟦agreement*|selvittävät|selvittää⟧ ensin, ⟪grammar_ok|onko kyse⟫ tunnetusta viasta. Tunnetut viat ⟪grammar_ok|on listattu⟫ ⟪product|Confluencessa⟫. Jos vika on uusi, ⟪grammar_ok|siitä riippuu⟫, ohjataanko pyyntö kehitystiimille.

Asiakkaalle ei luvata korjausaikaa, ⟦vaan_vain*|vain|vaan⟧ kerrotaan, milloin seuraava tilannetieto tulee. Vastaus kirjoitetaan ⟪grammar_ok|selkeämmin kuin⟫ sisäinen kommentti. Kun ⟦punctuation|pyyntö on ratkaistu asiakkaalta|pyyntö on ratkaistu, asiakkaalta⟧ pyydetään vahvistus.

Ratkaisemattomat ⟪grammar_ok|pyynnöt, joita⟫ ei ole päivitetty ⟪unit|viikkoon⟫, nousevat ⟦capitalization|Maanantain|maanantain⟧ palaveriin. ⟪grammar_ok|Kaksi tukihenkilöä käy⟫ ne läpi yhdessä.

## Eskalointi

Pyyntö eskaloidaan ⟪grammar_ok|kehitystiimille, jos⟫ ratkaisu vaatii koodimuutoksen. Eskaloinnissa liitetään mukaan lokit, ⟦joka_jotka*|jotka|joka⟧ on kerätty ⟪product|Grafanasta⟫ ⟪grammar_ok|samalla kun⟫ vikaa tutkittiin. Kehitystiimi vastaa ⟪unit|kahden arkipäivän⟫ kuluessa; jos se ⟦punctuation|ei vastaa tukihenkilö|ei vastaa, tukihenkilö⟧ muistuttaa ⟪identifier|#alusta⟫-kanavalla.

⟪grammar_ok|Sekä asiakas että⟫ tukihenkilö näkevät tiketin tilan portaalissa. ⟪foreign|"Escalated to engineering" means the ticket has left the support queue.⟫ Tämä ⟪grammar_ok|kertoo siitä⟫, että ⟪grammar_ok|prosessi on läpinäkyvä⟫, ⟦joka_jotka*|joka|mikä⟧ vähentää kyselyjä.

Ohjeen omistaa tukipäällikkö ⟪name|Anni Swan⟫. Muutokset ⟦agreement*|hyväksyy|hyväksytään⟧ tukitiimin viikkopalaverissa.
