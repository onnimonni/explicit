# Kapasiteettisuunnitelma 2027

Suunnitelma kertoo siitä, miten alustan kapasiteetti riittää ensi vuoden kasvuun. Se perustuu sitä, että käyttäjämäärä kasvaa 30 prosenttia ja varaukset 40 prosenttia.

## Nykytila

Alusta pyörii Kubernetes-klusterissa, jossa on 12 solmua. Solmut on 80-prosenttisesti käytössä ruuhka-aikaan. Tietokanta kestää nykyisen kuorman, mutta levytila loppuu kesäkuussa, joka on suurin yksittäinen riski.

Viime vuonna kapasiteetti loppuivat kahdesti. Molemmat tapaukset johtui siitä, että eräajot ajettiin ruuhka-aikaan. Emme olleet varautuneet sitä, että eräajot kasvavat käyttäjämäärän mukana.

## Ennuste

| Resurssi | Nyt | Ennuste 12/2027 | Toimenpide |
|----------|-----|-----------------|-----------|
| Solmut | 12 | 18 | Lisätään Maaliskuussa |
| Levytila (PostgreSQL) | 1,4 TiB | 2,6 TiB | Laajennus toukokuussa |
| Jonon läpimeno | 2 000/h | 3 500/h | Ei toimenpiteitä |
| Objektitallennus | 18 TiB | 30 TiB | Elinkaarisääntö |

Ennuste on tehty kolmen vuoden datasta. Luvut ei sisällä mahdollista uutta hyvinvointialuetta, jotka liittyisi alustaan syksyllä. Jos se liittyy luvut kaksinkertaistuvat.

## Toimenpiteet

1. Solmuja lisätään kuusi kappaletta. Kustannus on 48 000 € vuodessa; se on vähemmän kuin yhden katkon hinta.
2. Eräajot siirretään yöhön. Ajot, joka eivät siedä siirtoa, jaetaan pienempiin osiin.
3. Levytilan hälytysraja lasketaan 70 prosenttiin. Kukaan ei enää luota siihen, että 95 prosentin raja riittää.
4. Objektitallennukseen tehdään elinkaarisääntö, joka siirtää yli vuoden vanhat tiedostot kylmään tasoon.

Toimenpiteet ei maksa paljon, mutta ne vaativat huoltokatkon. Katko kestää kaksi tuntia, joka on hyväksyttävää Sunnuntaina aamuyöllä.

## Riskit

Suurin riski ei ole raha vain aika: solmujen toimitusaika on kahdeksan viikkoa. Toimittaja varoitti sitä, että tammikuun tilaukset ehtii perille vasta Maaliskuussa. Tilaus tehdään siksi heti, kun budjetti on hyväksytty.

Toinen riski on osaaminen. Vain kaksi henkilöä osaa laajentaa tietokannan levytilaa. Sitä syystä toimenpide dokumentoidaan ajokirjaan ja harjoitellaan staging-ympäristössä ennen kuin se tehdään tuotannossa. Toimittajan arvio englanniksi: The disk expansion is an online operation and should not require downtime if the replica is promoted first. Emme luota siihen sokeasti, vaan testaamme sen.

Suunnitelman laativat Ville Ranta ja Aino Kallas. Se hyväksytään ohjausryhmässä joulukuussa; ohjausryhmä päättää siitä, mitkä toimenpiteet tehdään ensin.
