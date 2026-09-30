# Varmuuskopiointipolitiikka

Politiikka koskee kaikkia tuotantojärjestelmiä, joka sisältävät asiakas- tai potilastietoja. Järjestelmät, jotka eivät sisällä henkilötietoja, noudattavat kevennettyä mallia.

## Periaatteet

1. Jokaisesta tietokannasta otetaan varmuuskopio vähintään kerran vuorokaudessa sekä jatkuva että päivittäinen kopio.
2. Kopiot säilytetään eri sijainnissa kuin alkuperäinen data. Pidämme siitä kiinni myös kehitysympäristöissä.
3. Palautus testataan neljännesvuosittain. Testaamaton kopio vaan näyttää turvalliselta.
4. Salausavaimet säilytetään Vaultissa, jonne pääsee vain päivystäjän roolilla.

Politiikka riippuu siitä, mihin luokkaan järjestelmä kuuluu. Luokittelusta päättävät tietohallintojohtaja.

## Säilytysajat

| Kohde | Tiheys | Säilytys |
|-------|--------|----------|
| PostgreSQL | jatkuva (WAL) + klo 01 | 35 päivää |
| Tiedostovarasto | 4 h | 90 päivää |
| Lokit | vuorokausi | 1 vuosi |
| Kubernetes-konfiguraatio | jokaisesta muutoksesta | 2 vuotta |

Säilytysaika on pidempi kun lain vaatima minimi, koska palautuspyynnöt tulee usein myöhässä. Kun säilytysaika päättyy, kopiot poistetaan automaattisesti.

## Vastuut

Päivystäjä tarkistaa joka aamu, että edellisen yön kopiot ovat onnistuneet. Epäonnistuneesta kopiosta ilmoitetaan sitä heti #hairiot-kanavalla, ja jos syytä ei löydy tunnissa, asia eskaloidaan.

Tietokantatiimi huolehtii siitä, että palautusharjoitus tehdään ja dokumentoidaan. Harjoituksesta kirjoitetaan raportti, jotka sisältää keston, ongelmat ja korjaukset. Osa harjoituksista tehdään ilman ennakkoilmoitusta.

Kehittäjät ei saa ottaa omia kopioita tuotantodatasta työasemille. Testidataa tarvitsevat kehittäjät käyttävät anonymisoitua staging-kopiota, joka riittää lähes aina.

## Poikkeamat

Jos kopio puuttuu yli kahdelta peräkkäiseltä päivältä, kyse on tieto turva poikkeamasta ja siitä tehdään ilmoitus tietosuojavastaavalle. Poikkeaman syy ei ole koskaan vain tekninen, vain myös prosessin puute.

Politiikka tarkistetaan Tammikuussa. Englanninkielinen tiivistelmä toimittajille: Backups are taken daily, retained for 35 days and restore-tested every quarter. Tiivistelmä on käännetty myös ruotsiksi vaikka sitä ei vaadita.
