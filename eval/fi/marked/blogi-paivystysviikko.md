---
lang: fi
author: Sanna Lahti
---

# Viikko päivystäjänä: mitä opin ensimmäisestä vuorostani

Aloitin elokuussa alustatiimissä ja syyskuun toisella viikolla oli ensimmäinen päivystysvuoroni. Kirjoitan tämän muistiksi itselleni ja avuksi seuraaville uusille.

## Ennen vuoroa

Luin ajokirjat läpi ja ⟦typo|tesasin|testasin⟧, että ⟪product|Opsgenie⟫ oikeasti herättää puhelimen. ⟦typo|Kollgea|Kollega⟧ sanoi ⟪colloquial|"kyl se herättää, älä huoli"⟫, mutta halusin varmistaa. Hyvä niin, koska ilmoitukset oli estetty puhelimen yötilassa.

Toinen asia, jonka olisin halunnut tietää etukäteen: ⟪acronym|VPN⟫-yhteyden muodostaminen kotoa kestää yllättävän kauan, jos ⟪product|Authenticator⟫ ei ole valmiiksi auki.

## Maanantai: hiljaista

Ei hälytyksiä. Käytin ajan ⟦compound_joined|siihenettä|siihen että⟧ kävin läpi edellisen viikon ⟦typo|hälytyshistoran|hälytyshistorian⟧. Huomasin, että sama levytilahälytys toistuu joka ⟦capitalization|Keskiviikko|keskiviikko⟧ ⟪unit|klo 3⟫. Kukaan ei ollut selvittänyt syytä.

## Tiistai: ensimmäinen hälytys

⟪unit|Klo 22.40⟫ ⟪identifier|`api_error_rate > 2%`⟫. Avasin ⟪product|Grafanan⟫ ja näin, että virheet tulivat yhdestä päätepisteestä. Lokista löytyi ⟪code|`context deadline exceeded`⟫ -virheitä ⟦compound_split|tieto kanta|tietokanta⟧-kutsuista.

```bash
kubectl logs -n sovellus deploy/varaus --since=15m | grep -c "deadline exceeded"
```

Ajokirjan mukaan ensimmäinen toimenpide on tarkistaa ⟪product|pgBouncerin⟫ yhteysvaranto. Se oli täynnä. Käynnistin ⟪product|pgBouncerin⟫ uudelleen ja virheet ⟦confusion*|loppuvat|loppuivat⟧ ⟪unit|kahdessa minuutissa⟫. Kirjoitin lyhyen raportin kanavalle ja menin nukkumaan.

Jälkikäteen ajateltuna olisi pitänyt katsoa, mistä varannon täyttyminen johtui. Se selvisi vasta torstaina: eräajo, joka avaa yhteyksiä ⟦compound_joined|eikäsulje|eikä sulje⟧ niitä.

## Keskiviikko: se levytilahälytys

Päätin selvittää ⟪unit|klo 3⟫:n hälytyksen. Syy oli yksinkertainen: varmuuskopiointi kirjoittaa väliaikaistiedoston samalle levylle, ennen kuin se siirretään ⟪product|S3⟫:een. Levy täyttyy hetkeksi ⟪unit|96 %⟫:iin ja hälytys laukeaa. Korjaus oli yhden rivin muutos: ⟪identifier|`TMPDIR=/mnt/scratch`⟫.

Tästä opin, että toistuvat hälytykset, joita kukaan ei ⟦punctuation|tutki ovat|tutki, ovat⟧ vaarallisimpia. Ne opettavat sivuuttamaan hälytykset.

## Perjantai: eskalointi

⟪unit|Klo 16.10⟫ ⟪name|Kanta⟫-integraatio alkoi palauttaa ⟪number|500⟫-virheitä. En löytänyt syytä ⟪unit|20 minuutissa⟫, joten eskaloin integraatiotiimille ajokirjan ohjeen mukaan. Syy oli ⟪name|Kannan⟫ päässä: heidän varmenteessa oli ongelma. Meidän ei tarvinnut tehdä mitään ⟦confusion*|kun|kuin⟧ odottaa ja tiedottaa.

Eskalointi tuntui aluksi epäonnistumiselta. Mentorini ⟪name|Juha Itkonen⟫ sanoi, että eskaloimatta jättäminen olisi ollut virhe, eskalointi ei.

## Mitä tekisin toisin

- Tarkistaisin puhelimen yötilan asetukset ennen vuoroa.
- Kirjoittaisin muistiinpanot heti, en seuraavana aamuna.
- Kysyisin ⟦compound_joined|useamminkuin|useammin kuin⟧ kerran, jos ajokirja ei ole selvä.

Kiitos tiimille tuesta. Ensi lokakuussa olen taas vuorossa, ja nyt tiedän paremmin, mitä odottaa.
