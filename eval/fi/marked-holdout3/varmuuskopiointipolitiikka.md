# Varmuuskopiointipolitiikka

Politiikka koskee kaikkia tuotantojärjestelmiä, ⟦joka_jotka*|joka|jotka⟧ sisältävät asiakas- tai potilastietoja. ⟪grammar_ok|Järjestelmät, jotka⟫ eivät sisällä henkilötietoja, noudattavat kevennettyä mallia.

## Periaatteet

1. Jokaisesta tietokannasta otetaan varmuuskopio ⟪grammar_ok|vähintään kerran⟫ vuorokaudessa ⟪grammar_ok|sekä jatkuva että⟫ päivittäinen kopio.
2. Kopiot säilytetään ⟪grammar_ok|eri sijainnissa kuin⟫ alkuperäinen data. ⟪grammar_ok|Pidämme siitä kiinni⟫ myös kehitysympäristöissä.
3. Palautus testataan ⟪unit|neljännesvuosittain⟫. Testaamaton kopio ⟦vaan_vain*|vaan|vain⟧ ⟪grammar_ok|näyttää⟫ turvalliselta.
4. Salausavaimet säilytetään ⟪product|Vaultissa⟫, jonne pääsee vain päivystäjän roolilla.

Politiikka ⟪grammar_ok|riippuu siitä⟫, mihin luokkaan järjestelmä kuuluu. Luokittelusta ⟦agreement*|päättävät|päättää⟧ tietohallintojohtaja.

## Säilytysajat

| Kohde | Tiheys | Säilytys |
|-------|--------|----------|
| ⟪product|PostgreSQL⟫ | jatkuva (⟪acronym|WAL⟫) + ⟪unit|klo 01⟫ | ⟪unit|35 päivää⟫ |
| Tiedostovarasto | ⟪unit|4 h⟫ | ⟪unit|90 päivää⟫ |
| Lokit | ⟪unit|vuorokausi⟫ | ⟪unit|1 vuosi⟫ |
| ⟪product|Kubernetes⟫-konfiguraatio | jokaisesta muutoksesta | ⟪unit|2 vuotta⟫ |

Säilytysaika on pidempi ⟦kun_kuin*|kun|kuin⟧ lain vaatima minimi, koska palautuspyynnöt ⟦agreement*|tulee|tulevat⟧ usein myöhässä. ⟪grammar_ok|Kun säilytysaika päättyy⟫, kopiot poistetaan automaattisesti.

## Vastuut

Päivystäjä tarkistaa ⟪grammar_ok|joka aamu⟫, että edellisen yön kopiot ovat onnistuneet. Epäonnistuneesta kopiosta ⟦sita_siita*|ilmoitetaan sitä|ilmoitetaan siitä⟧ heti ⟪identifier|#hairiot⟫-kanavalla, ⟦punctuation|ja jos|, ja jos⟧ syytä ei löydy ⟪unit|tunnissa⟫, asia eskaloidaan.

Tietokantatiimi ⟪grammar_ok|huolehtii siitä⟫, että palautusharjoitus tehdään ja dokumentoidaan. Harjoituksesta kirjoitetaan raportti, ⟦joka_jotka*|jotka|joka⟧ sisältää keston, ongelmat ja korjaukset. ⟪grammar_ok|Osa harjoituksista tehdään⟫ ilman ennakkoilmoitusta.

Kehittäjät ⟦agreement*|ei saa|eivät saa⟧ ottaa omia kopioita tuotantodatasta työasemille. Testidataa ⟪grammar_ok|tarvitsevat kehittäjät⟫ käyttävät anonymisoitua ⟪code|staging⟫-kopiota, ⟦joka_jotka*|joka|mikä⟧ riittää lähes aina.

## Poikkeamat

Jos kopio puuttuu yli ⟪unit|kahdelta⟫ peräkkäiseltä päivältä, kyse on ⟦compound_split|tieto turva poikkeamasta|tietoturvapoikkeamasta⟧ ja siitä tehdään ilmoitus tietosuojavastaavalle. Poikkeaman syy ei ole koskaan ⟪grammar_ok|vain tekninen⟫, ⟦vaan_vain*|vain|vaan⟧ ⟪grammar_ok|myös prosessin⟫ puute.

Politiikka tarkistetaan ⟦capitalization|Tammikuussa|tammikuussa⟧. Englanninkielinen tiivistelmä toimittajille: ⟪foreign|Backups are taken daily, retained for 35 days and restore-tested every quarter.⟫ Tiivistelmä on käännetty myös ruotsiksi ⟦punctuation|vaikka|, vaikka⟧ sitä ei vaadita.
