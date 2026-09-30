---
lang: fi-FI
---

# Perehdytysopas uusille kehittäjille

Tervetuloa alustatiimiin! Tämä opas kertoo, mitä ensimmäisen viikon aikana tapahtuu ja mistä löydät apua.

## Ensimmäinen päivä

Saat kannetavan, tunnukset ja YubiKeyn. Kirjaudu GitHubiin ja Slackiin työsähköpostillasi. Liity kanaville #alusta ja #hairiot. Tiimi lounastaa yhdessä klo 11.30; maanantaisin lounaan tarjoaa työnantaja.

Kehitysympäristö asennetaan Nixillä. Aja `nix develop` repositorion juuressa niin saat Go:n, PostgreSQL:n ja kubectl:n oikeissa versioissa. Jos Dockerilla on ongelmia macOS:llä, kysy Villeltä.

```bash
git clone git@github.com:example/alusta.git
cd alusta && nix develop
make test
```

## Ensimmäinen viikko

1. Lue arkkitehtuurikuvaus ja kaksi uusinta jälkiarvota.
2. Tee pieni muutos: korjaa kirjoitusvirhe tai lisää testi. Avaa vetopyyntö ja pyydä katselmointi.
3. Seuraa päivystäjää yhden päivän ajan. Sitä oppii eniten.
4. Osallistu Torstain suunnittelupalaveriin.

Vetopyynnöt katselmoidaan vuorokauden kuluessa. Katselmoijat kommentoi GitHubissa; älä ota kommentteja henkilökohtaisesti. Koodi katselmoidaan, ei koodin kirjoittajaa.

## Työtavat

- Haarat nimetään `ominaisuus/…` ja `korjaus/…`. Haara poistetaan yhdistämisen jälkeen.
- Kaikki muutokset tuotantoon menevät Argo CD:n kautta. Kukaan ei aja `kubectl apply` käsin.
- Dokumentaatio kirjoitetaan suomeksi; koodi, commit-viestit ja muuttuja nimet englanniksi.
- Salaisuudet säilytettään Vaultissa, ei koskaan repositoriossa.

Tiimin periaatteenä on, että virhe on oppimistilaisuus. Jälkiarviot ovat syyllistämättömiä. Kuten Mika sanoo: "kaikki on joskus pudottanu tuotantokannan, mä kaks kertaa".

## Mistä apua

| Aihe | Kenelle |
|------|---------|
| Tunnukset ja laitteet | IT-tuki, #it-tuki |
| Arkkitehtuuri | Aino Kallas |
| Tietokanta | Väinö Linna |
| Päivystys | viikon päivystäjä, ks. Opsgenie |

Perehdytyksen lopuksi kuukauden kuluttua pidetään palautekeskustelu esihenkilön kanssa. Kerro silloin rohkesti, mikä oppaassa oli epäselvää, vaan älä jää odottamaan siihen asti, jos jokin on pielessä.

Note for English speakers: the onboarding buddy programme runs in English as well; ask in #alusta.
