---
lang: fi
---

# Tukipyyntöjen käsittely

Tämä ohje kuvaa, miten Jiraan tulevat tukipyynnöt luokitellaan ja ratkaistaan. Ohje koskee kaikkia tukihenkilöitä, jotka päivystävät #tuki-kanavalla.

## Vastaanotto

Jokainen pyyntö kuitataan tunnin kuluessa. Kuittauksessa kerrotaan asiakkaalle sitä, kuka pyyntöä käsittelee ja milloin vastausta voi odottaa. Pidämme sitä tärkeänä, että asiakas ei jää epätietoiseksi.

Pyynnöt, joka koskevat potilastietoja, siirretään heti tieto suoja vastaavalle. Asiakkaita on usein liikkeellä samanaikaisesti saman vian vuoksi; silloin pyynnöt yhdistetään yhdeksi häiriötiketiksi.

## Luokittelu

| Luokka | Kuvaus | Tavoiteaika |
|--------|--------|-------------|
| Kriittinen | Palvelu ei toimi kenelläkään | 4 h |
| Korkea | Toiminto ei toimi osalla käyttäjistä | 1 arkipäivä |
| Normaali | Yksittäinen käyttäjä, kiertotie on | 3 arkipäivää |
| Matala | Toive tai paranusehdotus | ei aikarajaa |

Luokka valitaan sekä vaikutuksen että kiireellisyyden perusteella. Pyyntö on kriittinen vasta silloin, kun kiertotietä ei ole. Tukihenkilö ei saa päättää että pyyntö on matala vain siksi, että se on työläs.

## Ratkaisu

Tukihenkilö selvittävät ensin, onko kyse tunnetusta viasta. Tunnetut viat on listattu Confluencessa. Jos vika on uusi, siitä riippuu, ohjataanko pyyntö kehitystiimille.

Asiakkaalle ei luvata korjausaikaa, vain kerrotaan, milloin seuraava tilannetieto tulee. Vastaus kirjoitetaan selkeämmin kuin sisäinen kommentti. Kun pyyntö on ratkaistu asiakkaalta pyydetään vahvistus.

Ratkaisemattomat pyynnöt, joita ei ole päivitetty viikkoon, nousevat Maanantain palaveriin. Kaksi tukihenkilöä käy ne läpi yhdessä.

## Eskalointi

Pyyntö eskaloidaan kehitystiimille, jos ratkaisu vaatii koodimuutoksen. Eskaloinnissa liitetään mukaan lokit, jotka on kerätty Grafanasta samalla kun vikaa tutkittiin. Kehitystiimi vastaa kahden arkipäivän kuluessa; jos se ei vastaa tukihenkilö muistuttaa #alusta-kanavalla.

Sekä asiakas että tukihenkilö näkevät tiketin tilan portaalissa. "Escalated to engineering" means the ticket has left the support queue. Tämä kertoo siitä, että prosessi on läpinäkyvä, joka vähentää kyselyjä.

Ohjeen omistaa tukipäällikkö Anni Swan. Muutokset hyväksyy tukitiimin viikkopalaverissa.
