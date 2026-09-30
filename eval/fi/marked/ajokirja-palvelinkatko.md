---
lang: fi
---

# Ajokirja: sovelluspalvelimen katko

Tätä ajokirjaa käytetään, kun ⟪product|Grafana⟫ hälyttää ⟪identifier|`app_up == 0`⟫ tai kun ⟦compound_split|asiakas palvelu|asiakaspalvelu⟧ ilmoittaa, ettei sovellus vastaa.

## Ensimmäiset viisi minuuttia

1. Kuittaa hälytys ⟪product|Opsgenie⟫ssa, jotta muut ⟦typo|päivystäjtä|päivystäjät⟧ ⟦inflection|tietävat|tietävät⟧ sinun ⟦typo|tutikvan|tutkivan⟧ asiaa.
2. Tarkista tila komennolla ⟪code|`kubectl get pods -n sovellus`⟫.
3. Katso, onko kuormantasaaja terve: ⟪url|https://lb.example.fi/status⟫.

```bash
kubectl get pods -n sovellus
kubectl logs -n sovellus deploy/sovellus --since=10m | tail -n 200
kubectl describe pod -n sovellus -l app=sovellus | grep -A5 Events
```

Jos podit ovat tilassa ⟪code|`CrashLoopBackOff`⟫, siirry kohtaan ⟪ordinal|3. Sovellus kaatuu⟫. Jos ne ovat tilassa ⟪code|`Running`⟫ mutta eivät vastaa, jatka kohtaan ⟪ordinal|2. Sovellus jumissa⟫.

## 2. Sovellus jumissa

Tavallisin syy on yhteysvarannon täyttyminen. Tarkista avoimet tietokantayhteydet:

```sql
SELECT count(*), state FROM pg_stat_activity GROUP BY state;
```

Jos ⟪code|`idle in transaction`⟫ -tilassa on yli ⟪number|50⟫ yhteyttä, ⟦double_letter|käynistä|käynnistä⟧ ⟪product|pgBouncer⟫ uudelleen. Se katkaisee roikkuvat yhteydet eikä sovellusta tarvitse käynnistää uudelleen.

Muista, että ⟪product|pgBouncerin⟫ uudelleenkäynnistys aiheuttaa ⟪unit|2–3 sekunnin⟫ katkon kaikille asiakkaille. Älä tee sitä ruuhka-aikaan ⟦confusion*|vain|vaan⟧ hiljaisena ⟦inflection|aikanä|aikana⟧.

## 3. Sovellus kaatuu

Lue lokista viimeinen poikkeus ennen kaatumista. Yleisimmät syyt:

- Muisti loppuu: ⟪code|`OOMKilled`⟫ tapahtumissa. Nosta rajaa väliaikaisesti ⟪unit|4 GiB⟫:iin ja avaa tiketti.
- Migraatio epäonnistui: ⟦compound_split|tieto kanta|tietokanta⟧ on skeemaltaan eri versiota kuin sovellus. Palauta edellinen ⟦loan|imagen|levykuvan⟧ versio.
- Varmenne vanhentunut: ⟪identifier|`x509: certificate has expired`⟫. ⟦capitalization|uusi|Uusi⟧ varmenne ⟪product|cert-managerin⟫ kautta.

Palauta edellinen versio komennolla ⟪code|`kubectl rollout undo deploy/sovellus -n sovellus`⟫. Palautus kestää ⟪abbrev|n.⟫ ⟪unit|90 s⟫.

## Katkon jälkeen

Kirjoita ⟦loan|postmortem|jälkiarvio⟧ ⟪unit|kahden arkipäivän⟫ kuluessa. Pohja löytyy tiedostosta ⟪code|`docs/pohjat/jalkiarvio.md`⟫. Muista ⟦inflection|kirjatä|kirjata⟧ katkon kesto, vaikutus asiakkaisiin ja korjaavat toimet.

Katkot, jotka kestävät yli ⟪unit|30 minuuttia⟫, ⟦double_letter|raportoidan|raportoidaan⟧ maanantain palaverissa ⟪name|Kelan⟫ yhteyshenkilölle.

> Huom: älä koskaan aja ⟪code|`kubectl delete namespace sovellus`⟫ tuotannossa, ⟪colloquial|"vaikka kuinka tekis mieli"⟫ (lainaus edellisen päivystäjän muistiinpanoista).
