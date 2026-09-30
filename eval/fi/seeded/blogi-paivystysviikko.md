---
lang: fi
author: Sanna Lahti
---

# Viikko päivystäjänä: mitä opin ensimmäisestä vuorostani

Aloitin elokuussa alustatiimissä ja syyskuun toisella viikolla oli ensimmäinen päivystysvuoroni. Kirjoitan tämän muistiksi itselleni ja avuksi seuraaville uusille.

## Ennen vuoroa

Luin ajokirjat läpi ja tesasin, että Opsgenie oikeasti herättää puhelimen. Kollgea sanoi "kyl se herättää, älä huoli", mutta halusin varmistaa. Hyvä niin, koska ilmoitukset oli estetty puhelimen yötilassa.

Toinen asia, jonka olisin halunnut tietää etukäteen: VPN-yhteyden muodostaminen kotoa kestää yllättävän kauan, jos Authenticator ei ole valmiiksi auki.

## Maanantai: hiljaista

Ei hälytyksiä. Käytin ajan siihenettä kävin läpi edellisen viikon hälytyshistoran. Huomasin, että sama levytilahälytys toistuu joka Keskiviikko klo 3. Kukaan ei ollut selvittänyt syytä.

## Tiistai: ensimmäinen hälytys

Klo 22.40 `api_error_rate > 2%`. Avasin Grafanan ja näin, että virheet tulivat yhdestä päätepisteestä. Lokista löytyi `context deadline exceeded` -virheitä tieto kanta-kutsuista.

```bash
kubectl logs -n sovellus deploy/varaus --since=15m | grep -c "deadline exceeded"
```

Ajokirjan mukaan ensimmäinen toimenpide on tarkistaa pgBouncerin yhteysvaranto. Se oli täynnä. Käynnistin pgBouncerin uudelleen ja virheet loppuvat kahdessa minuutissa. Kirjoitin lyhyen raportin kanavalle ja menin nukkumaan.

Jälkikäteen ajateltuna olisi pitänyt katsoa, mistä varannon täyttyminen johtui. Se selvisi vasta torstaina: eräajo, joka avaa yhteyksiä eikäsulje niitä.

## Keskiviikko: se levytilahälytys

Päätin selvittää klo 3:n hälytyksen. Syy oli yksinkertainen: varmuuskopiointi kirjoittaa väliaikaistiedoston samalle levylle, ennen kuin se siirretään S3:een. Levy täyttyy hetkeksi 96 %:iin ja hälytys laukeaa. Korjaus oli yhden rivin muutos: `TMPDIR=/mnt/scratch`.

Tästä opin, että toistuvat hälytykset, joita kukaan ei tutki ovat vaarallisimpia. Ne opettavat sivuuttamaan hälytykset.

## Perjantai: eskalointi

Klo 16.10 Kanta-integraatio alkoi palauttaa 500-virheitä. En löytänyt syytä 20 minuutissa, joten eskaloin integraatiotiimille ajokirjan ohjeen mukaan. Syy oli Kannan päässä: heidän varmenteessa oli ongelma. Meidän ei tarvinnut tehdä mitään kun odottaa ja tiedottaa.

Eskalointi tuntui aluksi epäonnistumiselta. Mentorini Juha Itkonen sanoi, että eskaloimatta jättäminen olisi ollut virhe, eskalointi ei.

## Mitä tekisin toisin

- Tarkistaisin puhelimen yötilan asetukset ennen vuoroa.
- Kirjoittaisin muistiinpanot heti, en seuraavana aamuna.
- Kysyisin useamminkuin kerran, jos ajokirja ei ole selvä.

Kiitos tiimille tuesta. Ensi lokakuussa olen taas vuorossa, ja nyt tiedän paremmin, mitä odottaa.
