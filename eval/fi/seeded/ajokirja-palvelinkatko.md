---
lang: fi
---

# Ajokirja: sovelluspalvelimen katko

Tätä ajokirjaa käytetään, kun Grafana hälyttää `app_up == 0` tai kun asiakas palvelu ilmoittaa, ettei sovellus vastaa.

## Ensimmäiset viisi minuuttia

1. Kuittaa hälytys Opsgeniessa, jotta muut päivystäjtä tietävat sinun tutikvan asiaa.
2. Tarkista tila komennolla `kubectl get pods -n sovellus`.
3. Katso, onko kuormantasaaja terve: https://lb.example.fi/status.

```bash
kubectl get pods -n sovellus
kubectl logs -n sovellus deploy/sovellus --since=10m | tail -n 200
kubectl describe pod -n sovellus -l app=sovellus | grep -A5 Events
```

Jos podit ovat tilassa `CrashLoopBackOff`, siirry kohtaan 3. Sovellus kaatuu. Jos ne ovat tilassa `Running` mutta eivät vastaa, jatka kohtaan 2. Sovellus jumissa.

## 2. Sovellus jumissa

Tavallisin syy on yhteysvarannon täyttyminen. Tarkista avoimet tietokantayhteydet:

```sql
SELECT count(*), state FROM pg_stat_activity GROUP BY state;
```

Jos `idle in transaction` -tilassa on yli 50 yhteyttä, käynistä pgBouncer uudelleen. Se katkaisee roikkuvat yhteydet eikä sovellusta tarvitse käynnistää uudelleen.

Muista, että pgBouncerin uudelleenkäynnistys aiheuttaa 2–3 sekunnin katkon kaikille asiakkaille. Älä tee sitä ruuhka-aikaan vain hiljaisena aikanä.

## 3. Sovellus kaatuu

Lue lokista viimeinen poikkeus ennen kaatumista. Yleisimmät syyt:

- Muisti loppuu: `OOMKilled` tapahtumissa. Nosta rajaa väliaikaisesti 4 GiB:iin ja avaa tiketti.
- Migraatio epäonnistui: tieto kanta on skeemaltaan eri versiota kuin sovellus. Palauta edellinen imagen versio.
- Varmenne vanhentunut: `x509: certificate has expired`. uusi varmenne cert-managerin kautta.

Palauta edellinen versio komennolla `kubectl rollout undo deploy/sovellus -n sovellus`. Palautus kestää n. 90 s.

## Katkon jälkeen

Kirjoita postmortem kahden arkipäivän kuluessa. Pohja löytyy tiedostosta `docs/pohjat/jalkiarvio.md`. Muista kirjatä katkon kesto, vaikutus asiakkaisiin ja korjaavat toimet.

Katkot, jotka kestävät yli 30 minuuttia, raportoidan maanantain palaverissa Kelan yhteyshenkilölle.

> Huom: älä koskaan aja `kubectl delete namespace sovellus` tuotannossa, "vaikka kuinka tekis mieli" (lainaus edellisen päivystäjän muistiinpanoista).
