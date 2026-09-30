# ADR-002: Viestijonon käyttöönotto laboratoriotulosten välityksessä

Tila: ehdotus
Päiväys: 2026-09-12
Kirjoittaja: ⟪name|Eeva-Liisa Manner⟫

## Tausta

Laboratoriotulokset siirretään nykyisin synkronisesti suoraan potilastietojärjestelmään. Kun ⟦typo|kohdejärjsetelmä|kohdejärjestelmä⟧ on huollossa, tulokset ⟦inflection|jäävat|jäävät⟧ jonoon lähettävän järjestelmän muistiin ja ⟦typo|katovat|katoavat⟧ uudelleenkäynnistyksessä. Viime kesäkuussa menetimme näin ⟪number|412⟫ tulosta.

Ongelma on tunnistettu ⟪name|Valviran⟫ tarkastuksessa ja korjaus on kiireellinen.

## Vaatimukset

- Yksikään tulos ei saa kadota, vaikka vastaanottaja olisi poissa käytöstä ⟪unit|48 h⟫.
- Tulokset on välitettävä ⟦double_letter|järjestyksesä|järjestyksessä⟧, jossa ne on kuitattu laboratoriossa.
- Viestin sisältö on salattava ⟦compound_split|levy tilalla|levytilalla⟧ ja siirrossa.
- Ratkaisun on toimittava sekä ⟪product|Kubernetes⟫-ympäristössä että vanhoilla ⟪product|VMware⟫-palvelimilla.

## Vaihtoehdot

| Vaihtoehto | Edut | Haitat |
|-----------|------|--------|
| ⟪product|RabbitMQ⟫ | Tuttu tiimille, hyvä ⟦loan|dokumentatio|dokumentaatio⟧ | Klusterointi työlästä |
| ⟪product|Apache Kafka⟫ | Skaalautuu, säilyttää viestit | Raskas, vaatii ⟪product|ZooKeeperin⟫ tai ⟪product|KRaftin⟫ |
| ⟪product|NATS JetStream⟫ | Kevyt, helppo asentaa | Vähemmän kokemusta Suomessa |
| Tietokantapohjainen jono | Ei uusia komponentteja | Kuormittaa päätietokantaa |

Arvioimme jokaista vaihtoehtoa ⟪unit|kahden viikon⟫ koejaksolla. Testit ajettiin ⟪product|GitHub Actions⟫ -ympäristössä, ja tulokset on koottu hakemistoon ⟪code|`docs/adr/002/tulokset/`⟫.

## Päätösehdotus

Ehdotamme ⟪product|RabbitMQ⟫:ta ⟪product|quorum⟫-jonoilla. Se täyttää vaatimukset ja tiimi osaa ylläpitää sitä jo nyt. ⟪product|Kafka⟫ olisi teknisesti ⟦confusion*|parempi kun|parempi kuin⟧ ⟪product|RabbitMQ⟫ suurilla volyymeillä, mutta volyymimme on ⟪abbrev|n.⟫ ⟪unit|2 000 viestiä tunnissa⟫, mikä ei riitä perustelemaan monimutkaisuutta.

Ehdotus käsitellään arkkitehtuuriryhmässä perjantaina. Jos ehdotus hyväksytään, käyttöönotto alkaa lokakuussa ja päättyy viimeistään, kun kaikki laboratoriot on siirretty uuteen väylään.

## Avoimet kysymykset

1. Kuka omistaa jonon elinkaaren hallinnan tuotannossa?
2. Riittääkö ⟪product|RabbitMQ⟫:n oma salaus vai tarvitaanko ⟦loan|encryptaus|salaus⟧ sovellustasolla?
3. Miten vanhat, käsittelemättömät viestit siirretään uuteen jonoon?
