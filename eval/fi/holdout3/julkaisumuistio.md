---
lang: fi-FI
---

# Julkaisumuistio 3.1: mitä muuttuu ja miksi

Versio 3.1 julkaistaan Marraskuun 12. päivänä. Tämä muistio kertoo siitä, mitä käyttäjät huomaavat ja mitä ylläpidon on tehtävä ennen kuin päivitys asennetaan.

## Käyttäjille näkyvät muutokset

Kalenterinäkymä on uusittu. Uusi näkymä on nopeampi kuin vanha ja toimii myös näppäimistöllä. Värit, joka eivät täyttäneet kontrastivaatimusta, on vaihdettu. Osa käyttäjistä on testannut näkymää Syyskuusta alkaen.

Viestien liitteet voivat nyt olla 20 megatavua. Raja oli aiemmin 5 megatavua, joka esti kuvien lähettämisen suoraan puhelimesta. Liitteet tarkistetaan haittaohjelmien varalta ennen kuin ne tallennetaan.

Oirearvio ohjaa nyt suoraan ajanvaraukseen, jos arvio ei vaadi hoitajan soittoa. Muutos ei koske kiireellisiä oireita, jotka ohjataan edelleen päivystykseen. Sitä huolimatta hoitaja tarkistaa jokaisen arvion jälkikäteen.

## Ylläpidolle

Päivitys ei ole valinnainen vain pakollinen kaikille ympäristöille. Se vaatii PostgreSQL 16:n ja Kubernetes 1.30:n. Migraatiot kestää n. 20 minuuttia kahden miljoonan varauksen kannassa. Migraation aikana palvelu on luku-tilassa, mikä näkyy käyttäjille ilmoituksena.

Ennen päivitystä ota varmuuskopio ja tarkista, että pgBouncerin yhteys määrä on vähintään 200. Jos määrä on pienempi migraatio voi jäädä jumiin. Päivitys on testattu staging-ympäristössä, jotka vastaa tuotantoa.

```bash
kubectl -n sovellus set image deploy/sovellus sovellus=registry.example.fi/sovellus:3.1.0
kubectl -n sovellus rollout status deploy/sovellus
```

Jos päivitys epäonnistuu, palaa versioon 3.0.4 komennolla `kubectl rollout undo`. Migraatiot ovat taaksepäin yhteensopivia, joten tietokantaa ei tarvitse palauttaa.

## Tunnetut ongelmat

- Safarin vanhat versiot näyttävät kalenterin väärin, kun näyttö on kapeampi kun 360 pikseliä.
- Androidilla push-ilmoitus voi tulla kahdesti, jos sovellus on auki taustalla. Korjaus tulee versiossa 3.1.1.
- Ruotsinkielinen oirearvio puuttuu vielä; se julkaistaan Joulukuussa.

Ongelmat on kirjattu Jiraan. Kysymykset kanavalle #julkaisu tai Aino Kallakselle. Englanninkielinen versio muistiosta: Release 3.1 ships on 12 November; run the migrations during a maintenance window and keep pgBouncer's pool at 200 or above. Muistio on pidempi kuin yleensä, koska muutoksia on paljon eikä niitä haluttu jakaa kahteen julkaisuun.
