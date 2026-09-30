# ADR-004: Hakupalvelun valinta

Tila: ehdotus
Päiväys: 2026-10-20

## Tilanne

Käyttäjät valittavat siitä, että haku on hidas ja ei löydä kirjoitusvirheellisiä nimiä. Nykyinen haku perustuu sitä, että PostgreSQL:n `LIKE`-kyselyt ajetaan jokaisesta näppäinpainalluksesta. Kyselyt kestää ruuhka-aikaan yli sekunnin.

Tuoteomistaja huomautti sitä, että haku on käytetyin toiminto ammattilaisten näkymässä. Kaksi kolmesta ammattilaisesta käyttää sitä päivittäin.

## Vaihtoehdot

### PostgreSQL ja pg_trgm

Trigrammi-indeksi nopeuttaa `LIKE`-kyselyt alle 50 millisekuntiin ja sietää kirjoitusvirheitä. Ei uusia komponentteja. Rajoitus: indeksi ei ymmärrä taivutusmuotoja, joka on suomessa ongelma.

### Elasticsearch

Täysiverinen hakumoottori suomen kielen analysaattorilla. Tiimillä ei ole kokemusta sen ylläpidosta. Kustannukset olivat pilotissa 1 200 € kuukaudessa, joka on enemmän kuin arvioimme.

### Meilisearch

Kevyt ja nopea. Suomen tuki on heikompi kun Elasticsearchissa, vaan riittää nimihakuun. Pilotissa se vastasi alle 20 millisekunnissa.

## Vertailu

| Kriteeri | pg_trgm | Elasticsearch | Meilisearch |
|----------|---------|---------------|-------------|
| Vasteaika | 50 ms | 30 ms | 20 ms |
| Taivutusmuodot | ei | kyllä | osittain |
| Ylläpito | ei lisätyötä | paljon | vähän |
| Kustannus | 0 | 1 200 €/kk | 150 €/kk |

Vertailun tulokset on koottu hakemistoon `docs/adr/004/`. Pilotit ajettiin samalla datalla ja samoilla kyselyillä.

## Päätösehdotus

Ehdotamme Meilisearchia. Perustelut:

1. Se ratkaisee sen ongelman, josta käyttäjät valittavat, eli hitauden ja kirjoitusvirheet.
2. Ylläpito on kevyttä: yksi kontti ja yksi levy.
3. Kustannus on kymmenesosa Elasticsearchin kustannuksesta.

Taivutusmuotojen puute ei ole este vaan rajoite: nimet eivät taivu haussa juuri koskaan. Jos tarve muuttuu vaihto Elasticsearchiin on mahdollinen, koska hakurajapinta on abstrahoitu.

Arkkitehtuuriryhmä käsittelevät ehdotuksen Marraskuun kokouksessa. Tuoteomistaja on tietoinen sitä, että käyttöönotto kestää kuusi viikkoa. Sitä huolimatta hän kannattaa ehdotusta. Toimittajan kommentti englanniksi: "Meilisearch handles Finnish names well enough as long as you enable the typo tolerance for words of five characters or more."

## Seuraukset

- Hakuindeksi rakennetaan uudelleen joka yö ja jos rakennus epäonnistuu, edellinen indeksi jää käyttöön.
- Henkilötunnuksia ei indeksoida. Haku toimii vain nimellä ja syntymäajalla.
- Kehittäjät, joka ylläpitävät hakua, saavat koulutuksen joulukuussa.
