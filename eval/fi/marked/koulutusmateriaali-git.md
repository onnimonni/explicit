---
lang: fi
---

# Git-peruskurssi: haarat ja yhdistäminen

Tämä materiaali on tarkoitettu ⟪product|Git⟫-peruskurssin ⟪ordinal|2. päivälle⟫. Oletamme, että osaat jo tehdä ⟪product|commitin⟫ ja työntää sen ⟦typo|etävarsatoon|etävarastoon⟧.

## Miksi haaroja käytetään

Haara on kevyt ⟦typo|osotin|osoitin⟧ ⟪product|committiin⟫. Se mahdollistaa työskentelyn erillään päähaarasta ⟦compound_joined|niinettä|niin että⟧ keskeneräinen työ ei riko muiden ympäristöä. Haaran luominen ⟦typo|kestä|kestää⟧ millisekunteja, ⟦confusion*|kun|kuin⟧ taas vanhoissa versionhallinnoissa se saattoi kestää minuutteja.

```bash
git switch -c ominaisuus/ajanvaraus-suodatin
git add src/suodatin.go
git commit -m "Lisää ajanvarauksen suodatin"
git push -u origin ominaisuus/ajanvaraus-suodatin
```

Haaran nimi ⟦punctuation|kertoo mitä|kertoo, mitä⟧ siinä tehdään. Käytämme etuliitteitä ⟪code|`ominaisuus/`⟫, ⟪code|`korjaus/`⟫ ja ⟪code|`kokeilu/`⟫.

## Yhdistäminen

Kun työ on valmis, haara yhdistetään ⟪code|`main`⟫-haaraan vetopyynnöllä. ⟪product|GitHub⟫ tarjoaa kolme tapaa:

| Tapa | Milloin |
|------|---------|
| ⟪code|Merge commit⟫ | Kun haaran historia halutaan ⟦inflection|säilyttäa|säilyttää⟧ sellaisenaan |
| ⟪code|Squash and merge⟫ | Kun haarassa on paljon pieniä ⟪product|committeja⟫ |
| ⟪code|Rebase and merge⟫ | Kun halutaan ⟦inflection*|lineaarisen|lineaarinen⟧ historia |

Tiimimme käyttää oletuksena ⟪code|Squash and merge⟫ -tapaa. ⟦capitalization|poikkeuksista|Poikkeuksista⟧ sovitaan katselmoinnissa.

## Ristiriidat

Ristiriita syntyy, kun kaksi haaraa on muuttanut samoja rivejä. ⟪product|Git⟫ merkitsee ristiriitaiset kohdat tiedostoon:

```text
<<<<<<< HEAD
func Suodata(ajat []Aika) []Aika {
=======
func Suodata(ajat []Aika, alku time.Time) []Aika {
>>>>>>> ominaisuus/ajanvaraus-suodatin
```

Ratkaise ristiriita muokkaamalla tiedostoa, poista merkit ja tee uusi commit. Älä koskaan ratkaise ristiriitaa valitsemalla sokeasti toista puolta ymmärtämättä, mitä muutokset tekevät.

## Harjoitus

1. Luo haara ⟪code|`harjoitus/<oma-nimi>`⟫.
2. Muokkaa tiedostoa ⟪code|`README.md`⟫ ja tee commit.
3. Pyydä pariasi muokkaamaan samaa riviä ⟪code|`main`⟫-haarassa.
4. Yritä yhdistää ja ratkaise ristiriita.

Harjoitukseen on varattu ⟪unit|45 minuuttia⟫. Jos jäät jumiin, kysy kouluttajalta tai katso ⟪url|https://git-scm.com/book/fi/v2⟫.

## Yleisiä virheitä

- Tehdään ⟪product|commit⟫ suoraan ⟪code|`main`⟫-haaraan. Suojaus estää tämän, mutta yritä silti muistaa vaihtaa haara.
- Haara jää elämään viikoiksi ilman yhdistämistä. Pitkäikäiset haarat aiheuttavat isoja ristiriitoja; yhdistä pieninä paloina.
- ⟪code|`git push --force`⟫ jaettuun haaraan. Käytä ⟪code|`--force-with-lease`⟫-⟦punctuation|valitsinta jos|valitsinta, jos⟧ pakotettu työntö on välttämätön.
- Salasanoja tai ⟪acronym|API⟫-avaimia päätyy historiaan. Ne on vaihdettava heti, ⟦compound_joined|vaikkakuinka|vaikka kuinka⟧ nopeasti commit poistettaisiin.

> ⟪colloquial|"Mä vaan pushasin ja nyt kaikki on rikki"⟫ on lause, jonka jokainen kouluttaja on kuullut. Se korjataan ⟦inflection|rauhassä|rauhassa⟧ ⟪code|`git reflog`⟫-komennolla.
