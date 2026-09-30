# ADR-002: Viestijonon käyttöönotto laboratoriotulosten välityksessä

Tila: ehdotus
Päiväys: 2026-09-12
Kirjoittaja: Eeva-Liisa Manner

## Tausta

Laboratoriotulokset siirretään nykyisin synkronisesti suoraan potilastietojärjestelmään. Kun kohdejärjsetelmä on huollossa, tulokset jäävat jonoon lähettävän järjestelmän muistiin ja katovat uudelleenkäynnistyksessä. Viime kesäkuussa menetimme näin 412 tulosta.

Ongelma on tunnistettu Valviran tarkastuksessa ja korjaus on kiireellinen.

## Vaatimukset

- Yksikään tulos ei saa kadota, vaikka vastaanottaja olisi poissa käytöstä 48 h.
- Tulokset on välitettävä järjestyksesä, jossa ne on kuitattu laboratoriossa.
- Viestin sisältö on salattava levy tilalla ja siirrossa.
- Ratkaisun on toimittava sekä Kubernetes-ympäristössä että vanhoilla VMware-palvelimilla.

## Vaihtoehdot

| Vaihtoehto | Edut | Haitat |
|-----------|------|--------|
| RabbitMQ | Tuttu tiimille, hyvä dokumentatio | Klusterointi työlästä |
| Apache Kafka | Skaalautuu, säilyttää viestit | Raskas, vaatii ZooKeeperin tai KRaftin |
| NATS JetStream | Kevyt, helppo asentaa | Vähemmän kokemusta Suomessa |
| Tietokantapohjainen jono | Ei uusia komponentteja | Kuormittaa päätietokantaa |

Arvioimme jokaista vaihtoehtoa kahden viikon koejaksolla. Testit ajettiin GitHub Actions -ympäristössä, ja tulokset on koottu hakemistoon `docs/adr/002/tulokset/`.

## Päätösehdotus

Ehdotamme RabbitMQ:ta quorum-jonoilla. Se täyttää vaatimukset ja tiimi osaa ylläpitää sitä jo nyt. Kafka olisi teknisesti parempi kun RabbitMQ suurilla volyymeillä, mutta volyymimme on n. 2 000 viestiä tunnissa, mikä ei riitä perustelemaan monimutkaisuutta.

Ehdotus käsitellään arkkitehtuuriryhmässä perjantaina. Jos ehdotus hyväksytään, käyttöönotto alkaa lokakuussa ja päättyy viimeistään, kun kaikki laboratoriot on siirretty uuteen väylään.

## Avoimet kysymykset

1. Kuka omistaa jonon elinkaaren hallinnan tuotannossa?
2. Riittääkö RabbitMQ:n oma salaus vai tarvitaanko encryptaus sovellustasolla?
3. Miten vanhat, käsittelemättömät viestit siirretään uuteen jonoon?
