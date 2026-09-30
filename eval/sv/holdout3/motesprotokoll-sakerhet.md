# Säkerhetsgruppens möte 9/2026

Tid: 7.10.2026 kl. 13–14
Närvarande: Aleksis Kivi (ordf.), Aino Kallas, Sirkka Selja, Ville Ranta (sekr.)

## 1. Sårbarhetsrapport

Kallas presenterade Trivys rapport. Det fanns tre kritiska sårbarheter, som alla gällde den gamla versionen av OpenSSL. Två av dem har redan rättats; den tredje rättas på Fredag.

Selja påpekade att rapporten också listar utvecklingsmiljöns containrar, vilket blåser upp siffrorna. Det beslöts att bara produktionscontainrar ta med i rapporten. Utvecklingsmiljön rapporteras separat så att dem inte glöms bort.

## 2. Resultat av penetrationstestet

Den externa leverantören hittade två medelallvarliga fynd. Det ena gällde att ett sessionsidentifierare inte byttes vid inloggning. Det andra visade att felsidorna avslöjat serverversionen.

Båda har rättats. Ranta frågade om leverantören har kontrollera rättelserna, och Kallas svarade att omtestet görs i slutet av Oktober. Leverantörens rapport är på engelska: "Both findings were remediated; a re-test is scheduled for the last week of October."

## 3. Lösenordspolicy

Det föreslogs att lösenordets minsta längd höjs till 16 tecken. Selja tyckte att förslaget var bättre än ett obligatoriskt bytesintervall, som avskaffades förra året. Användarna motsätta sig vanligen ändringar, men denna ändring syns inte för dem eftersom de flesta använder en lösenordshanterare.

Beslut: minsta längd är 16 tecken från 1.12. Gamla lösenord fungerar tills användaren byter de. Ändringen meddela på intranätet.

## 4. Tvåfaktorsautentisering

Alla anställda som har fjärråtkomst använder redan Authenticator. Kvar finns de delade konton som används i utrustning. Ranta utreder om dem kan ersättas med YubiKey.

Problemet är inte tekniskt utan administrativt: ingen äger de delade kontona. Att kontona är delade är en avvikelse från policyn. Vi borde att åtgärda det innan revisionen.

## 5. Övriga ärenden

- Säkerhetsutbildningen hålls i November. Deltagandet är obligatoriskt för alla, även cheferna.
- Nästa möte 4.11.2026. Protokollet godkänns per e-post om inga anmärkningar kommer inom tre dagar.

Mötet avslutades kl. 13.55. Protokollet skrev av Ville Ranta; det är kortare än vanligt eftersom två ärenden flyttades till nästa möte. De som vill kan läsa bilagorna i Confluence.
