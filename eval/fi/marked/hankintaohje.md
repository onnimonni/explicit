---
lang: fi
---

# Ohje: ohjelmistohankinnat ja kilpailutus

Ohje koskee kaikkia ohjelmistojen ja pilvipalvelujen hankintoja, joiden ⟦typo|arvonlisävrroton|arvonlisäveroton⟧ arvo ylittää ⟪unit|10 000 €⟫ sopimuskaudella. Pienemmät hankinnat tehdään ⟦double_letter|tietohallinon|tietohallinnon⟧ puitesopimuksella ilman ⟦typo|kilpailutsuta|kilpailutusta⟧.

## Kynnysarvot

| Arvo (alv 0 %) | Menettely |
|----------------|-----------|
| alle ⟪unit|10 000 €⟫ | Suorahankinta, kolme tarjousta suositeltavaa |
| ⟪unit|10 000–60 000 €⟫ | Kevennetty kilpailutus ⟪product|Cloudiassa⟫ |
| yli ⟪unit|60 000 €⟫ | Kansallinen hankintailmoitus ⟪name|Hilmassa⟫ |
| yli ⟪unit|221 000 €⟫ | ⟪acronym|EU⟫-kynnysarvo, ilmoitus ⟪name|TED⟫-tietokantaan |

Sopimuskauden arvo lasketaan koko ⟦inflection*|sopimuskaudelle|sopimuskaudelta⟧ optiot mukaan lukien. Hankintaa ei saa pilkkoa kynnysarvon kiertämiseksi.

## Tarvekartoitus

Ennen kilpailutusta hankintayksikkö laatii tarvekuvauksen, jossa vastataan ainakin seuraaviin kysymyksiin:

- Mitä ongelmaa ratkaistaan ja ketkä ovat käyttäjät?
- Onko olemassa avoimen lähdekoodin vaihtoehto, ⟦confusion*|joka|jota⟧ voidaan käyttää sellaisenaan?
- Miten ratkaisu ⟦typo|integrotuu|integroituu⟧ nykyisiin järjestelmiin (⟪name|Kanta⟫, ⟪product|Populus⟫, potilastietojärjestelmä)?
- Missä tietoja käsitellään ja siirtyykö niitä ⟪acronym|EU⟫:n ulkopuolelle?

Tietohallinto ja tietosuojavastaava lausuvat kuvauksesta ⟪unit|kahden viikon⟫ kuluessa. Ilman puoltavaa lausuntoa kilpailutusta ei aloiteta.

## Vaatimusmäärittely

Vaatimukset jaetaan pakollisiin ja pisteytettäviin. Pakollisia vaatimuksia ovat aina:

1. ⟪acronym|WCAG⟫ 2.1 AA -saavutettavuus
2. ⟦compound_joined|suomenja|suomen ja⟧ ruotsin kielen tuki käyttöliittymässä
3. tietojen säilytys ⟪acronym|EU⟫- tai ⟪acronym|ETA⟫-alueella
4. ⟪acronym|SSO⟫-kirjautuminen ⟪product|Entra ID⟫:llä
5. rajapinta tietojen vientiin avoimessa muodossa sopimuksen päättyessä

Pisteytettäviä vaatimuksia painotetaan enintään ⟪unit|40 %⟫ kokonaispisteistä; hinnan paino on vähintään ⟪unit|40 %⟫. ⟦typo|Elinkaarikustanukset|Elinkaarikustannukset⟧ lasketaan ⟪unit|neljälle vuodelle⟫.

Vältä ⟦loan|vendor lock-inia|toimittajalukkoa⟧: vaadi standardirajapintoja (⟪acronym|REST⟫, ⟪acronym|HL7 FHIR⟫) ja ⟦loan|documentaatiota|dokumentaatiota⟧ englanniksi tai suomeksi.

## Tarjousten vertailu

Vertailu tehdään ⟦compound_split|vertailu taulukolla|vertailutaulukolla⟧, jonka kaksi henkilöä täyttää toisistaan riippumatta. Erot käydään läpi yhdessä. Tarjoajille annetaan ⟦compound_joined|tarvittaessamahdollisuus|tarvittaessa mahdollisuus⟧ täsmentää tarjoustaan; täsmennys ei saa muuttaa tarjouksen sisältöä.

Hankintapäätöksen tekee ⟦inflection|tietohallintojohtajä|tietohallintojohtaja⟧ ⟪unit|60 000 €⟫:oon asti ja sen ylittävistä hyvinvointialueen hallitus. Päätös perustellaan kirjallisesti ja lähetetään kaikille tarjoajille. Valitusaika markkinaoikeuteen on ⟪unit|14 päivää⟫.

## Sopimus

Sopimuksessa käytetään ⟪acronym|JIT 2015⟫ -ehtoja. Lisäksi sovitaan:

- palvelutaso (vähintään ⟪unit|99,5 %⟫ kuukausittain) ja sanktiot sen alittamisesta
- tietojenkäsittelysopimuksesta (⟪acronym|GDPR⟫ 28 artikla)
- irtisanomisaika ⟪unit|6 kuukautta⟫ ja tietojen palautus ⟪unit|30 päivässä⟫
- hinnankorotusten ⟦inflection*|sidonnaisuus|sidonnaisuudesta⟧ indeksiin

Allekirjoitettu sopimus tallennetaan ⟪product|Dynastyyn⟫. Sopimuksen omistaja seuraa palvelutasoa ⟪unit|neljännesvuosittain⟫ ja ⟦typo|raportio|raportoi⟧ poikkeamista tietohallintojohtajalle.

Lisätietoja: ⟪name|Aleksis Kivi⟫, hankinta-asiantuntija, ⟪url|hankinnat@example.fi⟫. Ohje päivitetty ⟦capitalization|Kesäkuussa|kesäkuussa⟧ 2026.
