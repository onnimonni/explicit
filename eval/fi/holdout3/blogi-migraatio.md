# Blogi: näin siirsimme tietokannan MongoDB:stä PostgreSQL:ään

Kun kirjoitimme ADR-001:n keväällä, pidimme siirtoa parin kuukauden projektina. Se kesti viisi. Tässä kirjoituksessa kerron sitä, mikä yllätti ja mitä tekisimme toisin.

## Miksi siirto tehtiin

Syyt on kirjattu ADR:ään, jotka löytyy repositoriosta. Lyhyesti: lisenssi, transaktiot ja se, että tiimi osasi PostgreSQL:ää paremmin kun MongoDB:tä. Kukaan ei vastustanut päätöstä, joka jälkikäteen ajatellen olisi pitänyt herättää epäilyksiä.

## Mikä yllätti

Tietomallin muunnos oli helpompi kun pelkäsimme. Vaikeinta oli data, jotka ei noudattanut omaa skeemaansa. Noin kaksi prosenttia dokumenteista sisälsivät kenttiä, joita kukaan ei muistanut. Ne eivät olleet roskaa, vaan vanhojen integraatioiden jäänteitä.

Toinen yllätys oli suorituskyky. Odotimme, että PostgreSQL olisi hitaampi kirjoituksissa, vaan se oli nopeampi, koska pgBouncer poisti yhteyksien avaamisen kustannuksen. Lukukyselyt oli aluksi hitaampia, kunnes indeksit oli viritetty.

Kolmas yllätys: ihmiset. Kaksi kehittäjää lähti kesken projektin, jotka venytti aikataulua kuukaudella. Emme olleet varautuneet siihen, että osaaminen oli niin harvojen varassa.

## Miten siirto tehtiin

Ajoimme molempia kantoja rinnakkain kuusi viikkoa. Kirjoitukset menivät molempiin, luvut vain vanhaan. Joka yö vertasimme kantoja ja kirjasimme erot Grafanaan. Kun erot olivat nollassa kaksi viikkoa käänsimme luvut uuteen kantaan.

```sql
SELECT count(*) FROM varaukset WHERE luotu >= now() - interval '1 day';
```

Vertailuskripti ei ollut nopea vain perusteellinen. Se oli Go:lla kirjoitettu ja se tarkisti kaikki rivit, ei otosta. Pidän sitä projektin parhaana päätöksenä. Otospohjainen vertailu olisi jättänyt huomaamatta virheet, jotka koskivat vain vanhoja rivejä.

## Mitä tekisimme toisin

1. Varaisimme aikaa datan siivoukseen ennen kuin aloitamme.
2. Dokumentoisimme vanhat integraatiot, vaikka ne olisivat poistuneet käytöstä.
3. Emme luottaisi siihen, että kaikki ovat paikalla koko projektin ajan.

Kollega Juha Itkonen tiivisti englanniksi "the migration was easy, the data was not", ja olen samaa mieltä. Siirto onnistui, mutta ei siksi, että suunnitelma oli hyvä, vaan siksi, että tiimi jaksoi verrata dataa joka yö. Projektista opittiin enemmän kun mistään kurssista; Maaliskuussa pidämme siitä esityksen kehittäjäpäivässä.
