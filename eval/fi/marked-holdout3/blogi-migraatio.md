# Blogi: näin siirsimme tietokannan MongoDB:stä PostgreSQL:ään

Kun kirjoitimme ADR-001:n keväällä, ⟪grammar_ok|pidimme siirtoa⟫ parin kuukauden projektina. Se kesti viisi. Tässä kirjoituksessa kerron ⟦sita_siita*|sitä|siitä⟧, mikä yllätti ja ⟪grammar_ok|mitä tekisimme⟫ toisin.

## Miksi siirto tehtiin

Syyt on kirjattu ADR:ään, ⟦joka_jotka*|jotka|joka⟧ löytyy repositoriosta. Lyhyesti: lisenssi, transaktiot ja se, että ⟪grammar_ok|tiimi osasi⟫ ⟪product|PostgreSQL:ää⟫ paremmin ⟦kun_kuin*|kun|kuin⟧ ⟪product|MongoDB:tä⟫. ⟪grammar_ok|Kukaan ei vastustanut⟫ päätöstä, ⟦joka_jotka*|joka|mikä⟧ jälkikäteen ajatellen olisi pitänyt herättää epäilyksiä.

## Mikä yllätti

Tietomallin muunnos oli helpompi ⟦kun_kuin*|kun|kuin⟧ pelkäsimme. Vaikeinta oli data, ⟦joka_jotka*|jotka|joka⟧ ei noudattanut omaa skeemaansa. Noin ⟪unit|kaksi prosenttia⟫ dokumenteista ⟦agreement*|sisälsivät|sisälsi⟧ kenttiä, joita ⟪grammar_ok|kukaan ei muistanut⟫. Ne ⟪grammar_ok|eivät olleet roskaa, vaan⟫ vanhojen integraatioiden jäänteitä.

Toinen yllätys oli suorituskyky. Odotimme, että ⟪product|PostgreSQL⟫ olisi hitaampi kirjoituksissa, ⟦vaan_vain*|vaan|mutta⟧ ⟪grammar_ok|se oli nopeampi⟫, koska ⟪product|pgBouncer⟫ ⟪grammar_ok|poisti yhteyksien⟫ avaamisen kustannuksen. Lukukyselyt ⟦agreement*|oli|olivat⟧ aluksi hitaampia, kunnes ⟪grammar_ok|indeksit oli⟫ viritetty.

Kolmas yllätys: ihmiset. ⟪grammar_ok|Kaksi kehittäjää lähti⟫ kesken projektin, ⟦joka_jotka*|jotka|mikä⟧ venytti aikataulua kuukaudella. ⟪grammar_ok|Emme olleet varautuneet siihen⟫, että ⟪grammar_ok|osaaminen oli⟫ niin harvojen varassa.

## Miten siirto tehtiin

Ajoimme molempia kantoja rinnakkain ⟪unit|kuusi viikkoa⟫. Kirjoitukset menivät molempiin, ⟪grammar_ok|luvut vain⟫ vanhaan. Joka yö vertasimme kantoja ja ⟪grammar_ok|kirjasimme erot⟫ ⟪product|Grafanaan⟫. Kun erot ⟦punctuation|olivat nollassa kaksi viikkoa käänsimme|olivat nollassa kaksi viikkoa, käänsimme⟧ luvut uuteen kantaan.

```sql
SELECT count(*) FROM varaukset WHERE luotu >= now() - interval '1 day';
```

Vertailuskripti ei ollut nopea ⟦vaan_vain*|vain|vaan⟧ perusteellinen. Se oli ⟪product|Go⟫:lla kirjoitettu ja ⟪grammar_ok|se tarkisti⟫ kaikki rivit, ei otosta. ⟪grammar_ok|Pidän sitä⟫ projektin parhaana päätöksenä. Otospohjainen vertailu ⟪grammar_ok|olisi jättänyt⟫ huomaamatta ⟪grammar_ok|virheet, jotka⟫ koskivat vain vanhoja rivejä.

## Mitä tekisimme toisin

1. ⟪grammar_ok|Varaisimme aikaa⟫ datan siivoukseen ⟪grammar_ok|ennen kuin⟫ aloitamme.
2. Dokumentoisimme vanhat integraatiot, vaikka ne olisivat poistuneet käytöstä.
3. ⟪grammar_ok|Emme luottaisi siihen⟫, että ⟪grammar_ok|kaikki ovat⟫ paikalla koko projektin ajan.

Kollega ⟪name|Juha Itkonen⟫ tiivisti englanniksi ⟪foreign|"the migration was easy, the data was not"⟫, ja ⟪grammar_ok|olen samaa mieltä⟫. Siirto onnistui, mutta ei siksi, että suunnitelma oli hyvä, ⟪grammar_ok|vaan siksi⟫, että ⟪grammar_ok|tiimi jaksoi⟫ verrata dataa joka yö. Projektista opittiin enemmän ⟦kun_kuin*|kun|kuin⟧ mistään kurssista; ⟦capitalization|Maaliskuussa|maaliskuussa⟧ pidämme siitä esityksen kehittäjäpäivässä.
