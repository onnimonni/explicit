---
lang: fi
---

# Ohje: ohjelmistohankinnat ja kilpailutus

Ohje koskee kaikkia ohjelmistojen ja pilvipalvelujen hankintoja, joiden arvonlisävrroton arvo ylittää 10 000 € sopimuskaudella. Pienemmät hankinnat tehdään tietohallinon puitesopimuksella ilman kilpailutsuta.

## Kynnysarvot

| Arvo (alv 0 %) | Menettely |
|----------------|-----------|
| alle 10 000 € | Suorahankinta, kolme tarjousta suositeltavaa |
| 10 000–60 000 € | Kevennetty kilpailutus Cloudiassa |
| yli 60 000 € | Kansallinen hankintailmoitus Hilmassa |
| yli 221 000 € | EU-kynnysarvo, ilmoitus TED-tietokantaan |

Sopimuskauden arvo lasketaan koko sopimuskaudelle optiot mukaan lukien. Hankintaa ei saa pilkkoa kynnysarvon kiertämiseksi.

## Tarvekartoitus

Ennen kilpailutusta hankintayksikkö laatii tarvekuvauksen, jossa vastataan ainakin seuraaviin kysymyksiin:

- Mitä ongelmaa ratkaistaan ja ketkä ovat käyttäjät?
- Onko olemassa avoimen lähdekoodin vaihtoehto, joka voidaan käyttää sellaisenaan?
- Miten ratkaisu integrotuu nykyisiin järjestelmiin (Kanta, Populus, potilastietojärjestelmä)?
- Missä tietoja käsitellään ja siirtyykö niitä EU:n ulkopuolelle?

Tietohallinto ja tietosuojavastaava lausuvat kuvauksesta kahden viikon kuluessa. Ilman puoltavaa lausuntoa kilpailutusta ei aloiteta.

## Vaatimusmäärittely

Vaatimukset jaetaan pakollisiin ja pisteytettäviin. Pakollisia vaatimuksia ovat aina:

1. WCAG 2.1 AA -saavutettavuus
2. suomenja ruotsin kielen tuki käyttöliittymässä
3. tietojen säilytys EU- tai ETA-alueella
4. SSO-kirjautuminen Entra ID:llä
5. rajapinta tietojen vientiin avoimessa muodossa sopimuksen päättyessä

Pisteytettäviä vaatimuksia painotetaan enintään 40 % kokonaispisteistä; hinnan paino on vähintään 40 %. Elinkaarikustanukset lasketaan neljälle vuodelle.

Vältä vendor lock-inia: vaadi standardirajapintoja (REST, HL7 FHIR) ja documentaatiota englanniksi tai suomeksi.

## Tarjousten vertailu

Vertailu tehdään vertailu taulukolla, jonka kaksi henkilöä täyttää toisistaan riippumatta. Erot käydään läpi yhdessä. Tarjoajille annetaan tarvittaessamahdollisuus täsmentää tarjoustaan; täsmennys ei saa muuttaa tarjouksen sisältöä.

Hankintapäätöksen tekee tietohallintojohtajä 60 000 €:oon asti ja sen ylittävistä hyvinvointialueen hallitus. Päätös perustellaan kirjallisesti ja lähetetään kaikille tarjoajille. Valitusaika markkinaoikeuteen on 14 päivää.

## Sopimus

Sopimuksessa käytetään JIT 2015 -ehtoja. Lisäksi sovitaan:

- palvelutaso (vähintään 99,5 % kuukausittain) ja sanktiot sen alittamisesta
- tietojenkäsittelysopimuksesta (GDPR 28 artikla)
- irtisanomisaika 6 kuukautta ja tietojen palautus 30 päivässä
- hinnankorotusten sidonnaisuus indeksiin

Allekirjoitettu sopimus tallennetaan Dynastyyn. Sopimuksen omistaja seuraa palvelutasoa neljännesvuosittain ja raportio poikkeamista tietohallintojohtajalle.

Lisätietoja: Aleksis Kivi, hankinta-asiantuntija, hankinnat@example.fi. Ohje päivitetty Kesäkuussa 2026.
