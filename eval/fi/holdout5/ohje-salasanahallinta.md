---
lang: fi-FI
---

# Ohje: salasanahallinnan käyttö

Kaikki työntekijät siirtyvät Bitwardeniin Marraskuun loppuun mennessä. Ohje kertoo siitä, miten holvi otetaan käyttöön ja mihin salasanat tallennetaan.

## Käyttöönotto

1. Kirjaudu osoitteessa https://vault.example.fi työsähköpostillasi.
2. Luo pääsalasana. Kiinnitä huomiota siihen, että se on pitkä; neljä satunnaista sanaa riittää.
3. Asenna selainlaajennus ja mobiilisovellus. Sovellukset synkronoituu automaattisesti.
4. Tutustu siihen, miten jaetut kokoelmat toimivat.

Pääsalasanaa ei voi palauttaa. Jos unohdat sen, holvi on menetetty, joka tarkoittaa, että kaikki salasanat on vaihdettava. Varaudu sitä tallentamalla palautuskoodi paperille.

## Mitä tallennetaan

Holviin tallennetaan kaikki työhön liittyvät tunnukset, joihin kuuluvat myös API-avaimet ja SSH-avainten salasanat. Henkilökohtaisia tunnuksia ei tallenneta työholviin, vain omaan holviin, jotka on erillinen.

Jaetut tunnukset tallennetaan tiimin kokoelmaan. Kokoelman omistaja on vastuussa sitä, että jäsenlista on ajan tasalla. Kun työntekijä lähtee omistaja poistaa hänet kokoelmasta samana päivänä.

## Mitä ei tallenneta

- Pääsalasanaa itseään. Se säilytetään vain päässä ja palautuskoodina kassakaapissa.
- Potilastietoja tai henkilötunnuksia. Holvi ei ole tietovarasto.
- Tuotannon juurisalasanoja. Ne on Vaultissa, jotka on eri järjestelmä samannimisyydestä huolimatta.

Sekaannus Bitwardenin ja Vaultin välillä on yleinen. Kyse ei ole vain nimestä vain käyttötarkoituksesta: toinen on ihmisille, toinen palveluille. Kehittäjät tottuvat sitä nopeasti.

## Yleisiä ongelmia

| Ongelma | Ratkaisu |
|---------|----------|
| Laajennus ei täytä kenttiä | Tarkista, että sivuston osoite vastaa tallennettua |
| Mobiilisovellus pyytää pääsalasanaa jatkuvasti | Ota käyttöön biometrinen avaus |
| Jaettu tunnus ei näy | Pyydä kokoelman omistajaa lisäämään sinut |

Ongelmat, joka eivät ratkea ohjeella, ilmoitetaan #it-tuki-kanavalla. Tuki reagoi sitä tunnin kuluessa arkisin. Tuki ei vaihda pääsalasanaa puolestasi, vaan ohjaa palautuskoodin käyttöön; se johtuu siitä, että tuki ei koskaan näe holvin sisältöä.

Vanhat salasanat, jotka on säilytetty selaimessa tai muistilapuilla, poistetaan siirron jälkeen. Selaimen salasanavarasto tyhjennetään keskitetysti joulukuussa; siitä mennessä jokaisen on siirrettävä omansa. Englanninkielinen pikaohje: Install the browser extension, create a long master password, and store the recovery code on paper. Pikaohje on lyhyempi kuin tämä sivu, mikä on tarkoitus.
