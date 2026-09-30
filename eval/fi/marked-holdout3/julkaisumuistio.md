---
lang: fi-FI
---

# Julkaisumuistio 3.1: mitä muuttuu ja miksi

Versio 3.1 julkaistaan ⟦capitalization|Marraskuun|marraskuun⟧ 12. päivänä. Tämä muistio ⟪grammar_ok|kertoo siitä⟫, mitä käyttäjät huomaavat ja mitä ylläpidon on tehtävä ⟪grammar_ok|ennen kuin⟫ päivitys asennetaan.

## Käyttäjille näkyvät muutokset

Kalenterinäkymä on uusittu. Uusi näkymä on ⟪grammar_ok|nopeampi kuin⟫ vanha ja toimii ⟪grammar_ok|myös näppäimistöllä⟫. Värit, ⟦joka_jotka*|joka|jotka⟧ eivät täyttäneet kontrastivaatimusta, on vaihdettu. ⟪grammar_ok|Osa käyttäjistä on⟫ testannut näkymää ⟦capitalization|Syyskuusta|syyskuusta⟧ alkaen.

Viestien liitteet voivat nyt olla ⟪unit|20 megatavua⟫. Raja oli aiemmin ⟪unit|5 megatavua⟫, ⟦joka_jotka*|joka|mikä⟧ ⟪grammar_ok|esti⟫ kuvien lähettämisen suoraan puhelimesta. Liitteet tarkistetaan haittaohjelmien varalta ⟪grammar_ok|ennen kuin⟫ ne tallennetaan.

Oirearvio ⟪grammar_ok|ohjaa⟫ nyt suoraan ajanvaraukseen, ⟪grammar_ok|jos arvio⟫ ei vaadi hoitajan soittoa. Muutos ei koske ⟪grammar_ok|kiireellisiä oireita, jotka⟫ ohjataan edelleen päivystykseen. ⟦sita_siita*|Sitä|Siitä⟧ huolimatta hoitaja tarkistaa jokaisen arvion jälkikäteen.

## Ylläpidolle

Päivitys ei ole valinnainen ⟦vaan_vain*|vain|vaan⟧ pakollinen kaikille ympäristöille. Se vaatii ⟪product|PostgreSQL⟫ 16:n ja ⟪product|Kubernetes⟫ 1.30:n. Migraatiot ⟦agreement*|kestää|kestävät⟧ ⟪abbrev|n.⟫ ⟪unit|20 minuuttia⟫ ⟪unit|kahden miljoonan⟫ varauksen kannassa. Migraation aikana palvelu on ⟪grammar_ok|luku-tilassa, mikä⟫ näkyy käyttäjille ilmoituksena.

Ennen päivitystä ota varmuuskopio ja ⟪grammar_ok|tarkista, että⟫ ⟪product|pgBouncerin⟫ ⟦compound_split|yhteys määrä|yhteysmäärä⟧ on vähintään ⟪number|200⟫. Jos ⟦punctuation|määrä on pienempi migraatio|määrä on pienempi, migraatio⟧ voi jäädä jumiin. Päivitys on testattu ⟪code|staging⟫-ympäristössä, ⟦joka_jotka*|jotka|joka⟧ vastaa tuotantoa.

```bash
kubectl -n sovellus set image deploy/sovellus sovellus=registry.example.fi/sovellus:3.1.0
kubectl -n sovellus rollout status deploy/sovellus
```

Jos päivitys epäonnistuu, palaa versioon 3.0.4 komennolla ⟪code|`kubectl rollout undo`⟫. Migraatiot ovat taaksepäin yhteensopivia, joten ⟪grammar_ok|tietokantaa ei tarvitse⟫ palauttaa.

## Tunnetut ongelmat

- ⟪product|Safarin⟫ vanhat versiot näyttävät kalenterin ⟪grammar_ok|väärin, kun⟫ näyttö on kapeampi ⟦kun_kuin*|kun|kuin⟧ ⟪unit|360 pikseliä⟫.
- ⟪product|Androidilla⟫ push-ilmoitus ⟪grammar_ok|voi tulla⟫ kahdesti, ⟪grammar_ok|jos sovellus⟫ on auki taustalla. Korjaus tulee versiossa 3.1.1.
- Ruotsinkielinen oirearvio ⟪grammar_ok|puuttuu vielä⟫; se julkaistaan ⟦capitalization|Joulukuussa|joulukuussa⟧.

Ongelmat on kirjattu ⟪product|Jiraan⟫. Kysymykset kanavalle ⟪identifier|#julkaisu⟫ tai ⟪name|Aino Kallakselle⟫. Englanninkielinen versio muistiosta: ⟪foreign|Release 3.1 ships on 12 November; run the migrations during a maintenance window and keep pgBouncer's pool at 200 or above.⟫ Muistio on ⟪grammar_ok|pidempi kuin⟫ yleensä, koska ⟪grammar_ok|muutoksia on⟫ paljon ⟦punctuation|eikä|, eikä⟧ niitä haluttu jakaa kahteen julkaisuun.
