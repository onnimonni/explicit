---
lang: sv
---

# Installationsguide

Den här guiden beskriver hur Kanta-proxyn installeras på en Ubuntu 24.04-server. Vi förutsätter att du har administratörsrättighter och att servern har minst 8 GiB minne.

## Förutsättningar

Kontrollera att följande program varor är installerade innan du börjar:

- PostgreSQL 16 eller senare
- Docker och Docker Compose
- OpenSSL 3.x

Installationen misslyckas om databasen inte är igång. Kontrollera statusen med `systemctl status postgresql`.

Vid första installationstillfället skapar systemet katallogen `/var/lib/proxy` och skriver en konfigurations fil dit. Redigera inte filen för hand, se avsnitt 4. Inställningar.

## Installation

1. Ladda ner paketet från https://example.se/nedladdningar/proxy-2.3.1.tar.gz.
2. Packa upp den i en katalog och gå dit.
3. Kör `./installera.sh --produktion`.

```console
curl -LO https://example.se/nedladdningar/proxy-2.3.1.tar.gz
tar xzf proxy-2.3.1.tar.gz
cd proxy-2.3.1 && ./installera.sh --produktion
```

Skriptet frågar efter databasens adress och inloggningsuppgifter. Lösenordet sparas inte i klartext utan skrivs till miljövariabeln `PROXY_DB_PASSWORD`.

När skriptet är klart startar tjänsten automatiskt. Du kan följa loggen med `journalctl -u proxy -f`.

## Inställningar

Inställningsfilen är i TOML-format. De viktigaste nycklarna:

| Nyckel | Beskrivning |
|--------|-------------|
| `listen_port` | Porten som servern lyssnar på. Standard 8443. |
| `db_url` | Databasens adress. |
| `log_level` | Loggnivå. Tillåtna värden: `debug`, `info`, `warn`. |

Efter att porten har ändrats måste tjänsten startas om. Ändringarna träder i kraft först därefer.

## Verifiering

Öppna https://localhost:8443/halsa i webbläsaren. Svaret bör vara `{"status":"ok"}`. Om svaret är ett fel, kontrollera brandväggens inställningar och se i loggen att certifikatet inte har gådd ut. Ett utgånget certifikat är en vanligare orsak än man tror.

Om problemet kvarstår, kontakta kundtjänsten eller öppna ett ärende på https://support.example.se. Supporten svarar måndag till fredag kl. 8–16.
