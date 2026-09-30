# Tietoturvaryhmän kokous 9/2026

Aika: 7.10.2026 klo 13–14
Läsnä: Aleksis Kivi (pj.), Aino Kallas, Sirkka Selja, Ville Ranta (siht.)

## 1. Haavoittuvuusraportti

Kallas esitteli Trivyn raportin. Kriittisiä haavoittuvuuksia oli kolme, joka kaikki koskivat OpenSSL:n vanhaa versiota. Kaksi niistä on jo korjattu; kolmas korjataan Perjantaina.

Selja huomautti, että raportti listaa myös kehitysympäristön kontit, jotka paisuttaa lukuja. Sovittiin, että raporttiin otetaan vain tuotantokontit. Kehitysympäristö raportoidaan erikseen jotta sen tila ei jää huomaamatta.

## 2. Tunkeutumistestauksen tulokset

Ulkopuolinen toimittaja löysi kaksi keskitason havaintoa. Toinen koski sitä, että istunto tunniste ei vaihtunut kirjautumisessa. Toinen kertoi sitä, että virhesivut paljastivat palvelinversion.

Molemmat on korjattu. Ranta kysyi, onko toimittaja tarkistanut korjaukset ja Kallas vastasi, että uusintatestaus tehdään Lokakuun lopussa. Toimittajan raportti on englanniksi: "Both findings were remediated; a re-test is scheduled for the last week of October."

## 3. Salasanapolitiikka

Ehdotettiin, että salasanan vähimmäispituus nostetaan 16 merkkiin. Selja piti ehdotusta parempana kun pakollista vaihtoväliä, josta luovuttiin viime vuonna. Käyttäjät vastustaa yleensä muutoksia, vaan tämä muutos ei näy heille, koska useimmat käyttävät salasanahallintaa.

Päätös: vähimmäispituus on 16 merkkiä 1.12. alkaen. Vanhat salasanat toimivat siihen asti, kun käyttäjä vaihtaa ne. Muutoksesta tiedotetaan sitä intranetissä.

## 4. Kaksivaiheinen tunnistautuminen

Kaikki työntekijät, joilla on etäyhteys, käyttävät jo Authenticatoria. Jäljellä ovat jaetut tunnukset, joita käytetään laitteissa. Ranta selvittävät, voidaanko ne korvata YubiKeyllä.

Ongelma ei ole tekninen vain hallinnollinen: kukaan ei omista jaettuja tunnuksia. Se, että tunnukset ovat jaettuja, jotka on sinänsä poikkeama politiikasta.

## 5. Muut asiat

- Tietoturva koulutus pidetään Marraskuussa. Osallistuminen on pakollista kaikille, myös esihenkilöille.
- Seuraava kokous 4.11.2026. Muistio hyväksytään sähköpostitse ellei huomautuksia tule kolmen päivän kuluessa.

Kokous päättyi klo 13.55. Muistion kirjoitti Ville Ranta; se on lyhyempi kuin tavallisesti, koska kaksi asiaa siirrettiin seuraavaan kokoukseen.
