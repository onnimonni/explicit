---
lang: fi
---

# Git-peruskurssi: haarat ja yhdistäminen

Tämä materiaali on tarkoitettu Git-peruskurssin 2. päivälle. Oletamme, että osaat jo tehdä commitin ja työntää sen etävarsatoon.

## Miksi haaroja käytetään

Haara on kevyt osotin committiin. Se mahdollistaa työskentelyn erillään päähaarasta niinettä keskeneräinen työ ei riko muiden ympäristöä. Haaran luominen kestä millisekunteja, kun taas vanhoissa versionhallinnoissa se saattoi kestää minuutteja.

```bash
git switch -c ominaisuus/ajanvaraus-suodatin
git add src/suodatin.go
git commit -m "Lisää ajanvarauksen suodatin"
git push -u origin ominaisuus/ajanvaraus-suodatin
```

Haaran nimi kertoo mitä siinä tehdään. Käytämme etuliitteitä `ominaisuus/`, `korjaus/` ja `kokeilu/`.

## Yhdistäminen

Kun työ on valmis, haara yhdistetään `main`-haaraan vetopyynnöllä. GitHub tarjoaa kolme tapaa:

| Tapa | Milloin |
|------|---------|
| Merge commit | Kun haaran historia halutaan säilyttäa sellaisenaan |
| Squash and merge | Kun haarassa on paljon pieniä committeja |
| Rebase and merge | Kun halutaan lineaarisen historia |

Tiimimme käyttää oletuksena Squash and merge -tapaa. poikkeuksista sovitaan katselmoinnissa.

## Ristiriidat

Ristiriita syntyy, kun kaksi haaraa on muuttanut samoja rivejä. Git merkitsee ristiriitaiset kohdat tiedostoon:

```text
<<<<<<< HEAD
func Suodata(ajat []Aika) []Aika {
=======
func Suodata(ajat []Aika, alku time.Time) []Aika {
>>>>>>> ominaisuus/ajanvaraus-suodatin
```

Ratkaise ristiriita muokkaamalla tiedostoa, poista merkit ja tee uusi commit. Älä koskaan ratkaise ristiriitaa valitsemalla sokeasti toista puolta ymmärtämättä, mitä muutokset tekevät.

## Harjoitus

1. Luo haara `harjoitus/<oma-nimi>`.
2. Muokkaa tiedostoa `README.md` ja tee commit.
3. Pyydä pariasi muokkaamaan samaa riviä `main`-haarassa.
4. Yritä yhdistää ja ratkaise ristiriita.

Harjoitukseen on varattu 45 minuuttia. Jos jäät jumiin, kysy kouluttajalta tai katso https://git-scm.com/book/fi/v2.

## Yleisiä virheitä

- Tehdään commit suoraan `main`-haaraan. Suojaus estää tämän, mutta yritä silti muistaa vaihtaa haara.
- Haara jää elämään viikoiksi ilman yhdistämistä. Pitkäikäiset haarat aiheuttavat isoja ristiriitoja; yhdistä pieninä paloina.
- `git push --force` jaettuun haaraan. Käytä `--force-with-lease`-valitsinta jos pakotettu työntö on välttämätön.
- Salasanoja tai API-avaimia päätyy historiaan. Ne on vaihdettava heti, vaikkakuinka nopeasti commit poistettaisiin.

> "Mä vaan pushasin ja nyt kaikki on rikki" on lause, jonka jokainen kouluttaja on kuullut. Se korjataan rauhassä `git reflog`-komennolla.
