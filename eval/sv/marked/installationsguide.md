---
lang: sv
---

# Installationsguide

Den här guiden beskriver hur ⟪product|Kanta⟫-proxyn installeras på en ⟪product|Ubuntu⟫ 24.04-server. Vi förutsätter att du har ⟦typo|administratörsrättighter|administratörsrättigheter⟧ och att servern har minst ⟪unit|8 GiB⟫ minne.

## Förutsättningar

Kontrollera att följande ⟦sarskrivning|program varor|programvaror⟧ är installerade innan du börjar:

- ⟪product|PostgreSQL⟫ 16 eller senare
- ⟪product|Docker⟫ och ⟪product|Docker Compose⟫
- ⟪product|OpenSSL⟫ 3.x

Installationen misslyckas om databasen inte är igång. Kontrollera statusen med ⟪code|`systemctl status postgresql`⟫.

Vid första installationstillfället skapar systemet ⟦double_consonant|katallogen|katalogen⟧ ⟪code|`/var/lib/proxy`⟫ och skriver en ⟦sarskrivning|konfigurations fil|konfigurationsfil⟧ dit. Redigera inte filen för hand, se avsnitt ⟪ordinal|4. Inställningar⟫.

## Installation

1. Ladda ner paketet från ⟪url|https://example.se/nedladdningar/proxy-2.3.1.tar.gz⟫.
2. Packa upp ⟦de_dem_gender*|den|det⟧ i en katalog och gå dit.
3. Kör ⟪code|`./installera.sh --produktion`⟫.

```console
curl -LO https://example.se/nedladdningar/proxy-2.3.1.tar.gz
tar xzf proxy-2.3.1.tar.gz
cd proxy-2.3.1 && ./installera.sh --produktion
```

Skriptet frågar efter databasens adress och inloggningsuppgifter. Lösenordet sparas inte i klartext utan skrivs till miljövariabeln ⟪identifier|`PROXY_DB_PASSWORD`⟫.

När skriptet är klart startar tjänsten automatiskt. Du kan följa loggen med ⟪code|`journalctl -u proxy -f`⟫.

## Inställningar

Inställningsfilen är i ⟪product|TOML⟫-format. De viktigaste nycklarna:

| Nyckel | Beskrivning |
|--------|-------------|
| ⟪identifier|`listen_port`⟫ | Porten som servern lyssnar på. Standard ⟪number|8443⟫. |
| ⟪identifier|`db_url`⟫ | Databasens adress. |
| ⟪identifier|`log_level`⟫ | Loggnivå. Tillåtna värden: ⟪code|`debug`⟫, ⟪code|`info`⟫, ⟪code|`warn`⟫. |

Efter att porten har ändrats måste tjänsten startas om. Ändringarna träder i kraft först ⟦typo|därefer|därefter⟧.

## Verifiering

Öppna ⟪url|https://localhost:8443/halsa⟫ i webbläsaren. Svaret bör vara ⟪code|`{"status":"ok"}`⟫. Om svaret är ett fel, kontrollera brandväggens inställningar och se i loggen att certifikatet inte har ⟦verb_form|gådd|gått⟧ ut. Ett utgånget certifikat är en vanligare orsak än man tror.

Om problemet kvarstår, kontakta kundtjänsten eller öppna ett ärende på ⟪url|https://support.example.se⟫. Supporten svarar måndag till fredag ⟪unit|kl. 8–16⟫.
