# Tietoturvaohje henkilöstölle

Tämä ohje koskee kaikkia, jotka käsittelevät hyvinvointialueen tietoja tai käyttävät sen laitteita. Ohje perustuu ⟪acronym|ISO 27001⟫ -standardiin ja ⟪name|Traficomin⟫ suosituksiin.

## Salasanat ja tunnistautuminen

Salasanan on oltava vähintään ⟪unit|14 merkkiä⟫ pitkä. Suosittelemme salalausetta, ⟪abbrev|esim.⟫ neljää satunnaista sanaa. Samaa salasanaa ei saa käyttää useassa palvelussa. Salasanahallinta ⟪product|Bitwarden⟫ on kaikkien käytössä.

Kaksivaiheinen tunnistautuminen on pakollinen kaikissa etäyhteyksissä. Käytä ⟪product|Authenticator⟫-sovellusta; tekstiviestikoodit ovat sallittuja vain varajärjestelynä.

Älä koskaan kerro salasanaasi kenellekään, et edes ⟪acronym|IT⟫-tuelle. ⟪acronym|IT-tuki⟫ ei kysy salasanoja.

## Laitteet

⟦typo|Työaseamt|Työasemat⟧ salataan ⟪product|BitLockerilla⟫ tai ⟪product|FileVaultilla⟫. Lukitse näyttö aina, kun poistut ⟦compound_split|työ pisteeltäsi|työpisteeltäsi⟧ (⟪code|Win+L⟫ tai ⟪code|Ctrl+Cmd+Q⟫). Kannettavaa ei saa ⟦double_letter|jättä|jättää⟧ autoon näkyville.

Omia laitteita ei saa liittää sisäverkkoon. Vierailijoille on erillinen ⟪identifier|`Vieras`⟫-verkko. ⟪acronym|USB⟫-muisteja saa käyttää vain, jos ne ovat tietohallinnon toimittamia ja salattuja.

## Sähköposti ja tietojenkalastelu

Tarkista lähettäjän osoite, ennen kuin avaat liitteen tai linkin. Epäilyttävät viestit ilmoitetaan ⟪product|Outlookin⟫ ⟪code|Ilmoita tietojenkalastelusta⟫ -painikkeella. Tietojenkalastelu on yleisin tapa, jolla hyökkääjät pääsevät sisään.

Tyypillisiä merkkejä:

- kiire ja uhkailu (⟪colloquial|"tilisi suljetaan tänään ellet"⟫)
- kirjoitusvirheet ja outo kieli
- linkki, jonka osoite eroaa näytetystä tekstistä
- pyyntö lähettää lahjakortteja tai muuttaa tilinumeroa

Potilastietoja ei saa lähettää tavallisella sähköpostilla. Käytä ⟪product|Turvapostia⟫ tai ⟪name|Kanta⟫-viestejä.

## Tietojen käsittely

Tiedot luokitellaan neljään luokkaan: julkinen, sisäinen, luottamuksellinen ja salassa pidettävä. Potilastiedot ovat aina salassa pidettäviä. Luottamuksellisia tietoja saa käsitellä vain ⟪compound|käyttöoikeushallinnan⟫ myöntämillä käyttöoikeuksilla ja niiden tulostamista tulee välttää.

⟦loan|Cloud-palveluita|Pilvipalveluita⟧ (⟪product|Google Drive⟫, ⟪product|Dropbox⟫ ⟪abbrev|jne.⟫) ei saa käyttää työtietojen tallentamiseen ilman tietohallinnon lupaa. Sallitut palvelut on lueteltu intranetissä.

## Poikkeamista ilmoittaminen

Ilmoita tietoturvapoikkeamasta ⟦inflection|välittömasti|välittömästi⟧ osoitteeseen ⟪url|tietoturva@example.fi⟫ tai ⟪number|puh. 1911⟫. Poikkeamia ovat ⟪abbrev|mm.⟫ kadonnut laite, väärälle henkilölle lähetetty viesti ja epäilty tietomurto. Siitä ei rangaista, että ilmoittaa omasta virheestään; ilmoittamatta jättämisestä voi seurata kurinpitotoimia.

Henkilötietojen tietoturvaloukkaus on ilmoitettava ⟪name|Tietosuojavaltuutetulle⟫ ⟪unit|72 tunnin⟫ kuluessa, joten nopea sisäinen ilmoitus on välttämätön. Tietosuojavastaava arvioi ilmoitusvelvollisuuden.

Ohje on päivitetty elokuussa 2026. Seuraava tarkistus helmikuussa 2027.
