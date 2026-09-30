---
lang: fi-FI
---

# Perehdytysopas uusille kehittäjille

Tervetuloa alustatiimiin! Tämä opas kertoo, mitä ⟪phrase|ensimmäisen viikon⟫ aikana tapahtuu ja mistä löydät apua.

## Ensimmäinen päivä

Saat ⟦typo|kannetavan|kannettavan⟧, tunnukset ja ⟪product|YubiKeyn⟫. Kirjaudu ⟪product|GitHubiin⟫ ja ⟪product|Slackiin⟫ työsähköpostillasi. Liity kanaville ⟪identifier|#alusta⟫ ja ⟪identifier|#hairiot⟫. ⟪phrase|Tiimi lounastaa⟫ yhdessä ⟪unit|klo 11.30⟫; maanantaisin lounaan tarjoaa työnantaja.

Kehitysympäristö asennetaan ⟪product|Nixillä⟫. Aja ⟪code|`nix develop`⟫ repositorion juuressa ⟦punctuation|niin|, niin⟧ saat ⟪product|Go⟫:n, ⟪product|PostgreSQL⟫:n ja ⟪product|kubectl⟫:n oikeissa versioissa. Jos ⟪product|Dockerilla⟫ on ongelmia ⟪product|macOS⟫:llä, kysy ⟪name|Villeltä⟫.

```bash
git clone git@github.com:example/alusta.git
cd alusta && nix develop
make test
```

## Ensimmäinen viikko

1. Lue arkkitehtuurikuvaus ja kaksi uusinta ⟦typo|jälkiarvota|jälkiarviota⟧.
2. Tee pieni muutos: korjaa kirjoitusvirhe tai lisää testi. Avaa vetopyyntö ja pyydä katselmointi.
3. Seuraa päivystäjää ⟪unit|yhden päivän⟫ ajan. ⟦confusion*|Sitä|Siitä⟧ oppii eniten.
4. Osallistu ⟦capitalization|Torstain|torstain⟧ suunnittelupalaveriin.

Vetopyynnöt katselmoidaan ⟪unit|vuorokauden⟫ kuluessa. Katselmoijat ⟦confusion*|kommentoi|kommentoivat⟧ ⟪product|GitHubissa⟫; älä ota kommentteja henkilökohtaisesti. ⟪phrase|Koodi katselmoidaan⟫, ei ⟪phrase|koodin kirjoittajaa⟫.

## Työtavat

- Haarat nimetään ⟪code|`ominaisuus/…`⟫ ja ⟪code|`korjaus/…`⟫. ⟪phrase|Haara poistetaan⟫ yhdistämisen jälkeen.
- Kaikki muutokset tuotantoon menevät ⟪product|Argo CD⟫:n kautta. Kukaan ei aja ⟪code|`kubectl apply`⟫ käsin.
- Dokumentaatio kirjoitetaan suomeksi; koodi, commit-viestit ja ⟦compound_split|muuttuja nimet|muuttujanimet⟧ englanniksi.
- Salaisuudet ⟦typo|säilytettään|säilytetään⟧ ⟪product|Vaultissa⟫, ei koskaan repositoriossa.

Tiimin ⟦inflection|periaatteenä|periaatteena⟧ on, että ⟪phrase|virhe on⟫ oppimistilaisuus. Jälkiarviot ovat syyllistämättömiä. Kuten ⟪name|Mika⟫ sanoo: ⟪colloquial|"kaikki on joskus pudottanu tuotantokannan, mä kaks kertaa"⟫.

## Mistä apua

| Aihe | Kenelle |
|------|---------|
| Tunnukset ja laitteet | ⟪acronym|IT⟫-tuki, ⟪identifier|#it-tuki⟫ |
| Arkkitehtuuri | ⟪name|Aino Kallas⟫ |
| Tietokanta | ⟪name|Väinö Linna⟫ |
| Päivystys | viikon päivystäjä, ks. ⟪product|Opsgenie⟫ |

Perehdytyksen lopuksi ⟪unit|kuukauden⟫ kuluttua pidetään palautekeskustelu esihenkilön kanssa. Kerro silloin ⟦typo|rohkesti|rohkeasti⟧, mikä oppaassa oli epäselvää, vaan älä jää odottamaan siihen asti, jos jokin on pielessä.

⟪foreign|Note for English speakers: the onboarding buddy programme runs in English as well; ask in #alusta.⟫
