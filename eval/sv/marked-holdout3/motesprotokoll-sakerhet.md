# Säkerhetsgruppens möte 9/2026

Tid: ⟪unit|7.10.2026 kl. 13–14⟫
Närvarande: ⟪name|Aleksis Kivi⟫ (ordf.), ⟪name|Aino Kallas⟫, ⟪name|Sirkka Selja⟫, ⟪name|Ville Ranta⟫ (sekr.)

## 1. Sårbarhetsrapport

⟪name|Kallas⟫ presenterade ⟪product|Trivys⟫ rapport. Det fanns ⟪number|tre⟫ kritiska sårbarheter, som alla gällde ⟪grammar_ok|den gamla⟫ versionen av ⟪product|OpenSSL⟫. Två av dem ⟪grammar_ok|har redan rättats⟫; den tredje rättas på ⟦capitalization|Fredag|fredag⟧.

⟪name|Selja⟫ påpekade att rapporten också listar utvecklingsmiljöns containrar, vilket blåser upp siffrorna. Det beslöts att bara produktionscontainrar ⟦present_tense*|ta|tas⟧ med i rapporten. Utvecklingsmiljön rapporteras separat så att ⟦de_dem*|dem|de⟧ inte glöms bort.

## 2. Resultat av penetrationstestet

Den externa leverantören hittade ⟪number|två⟫ medelallvarliga fynd. Det ena gällde att ⟦en_ett*|ett sessionsidentifierare|en sessionsidentifierare⟧ inte byttes vid inloggning. Det andra visade att felsidorna ⟦supine*|avslöjat|avslöjade⟧ serverversionen.

Båda har rättats. ⟪name|Ranta⟫ frågade om leverantören har ⟦supine*|kontrollera|kontrollerat⟧ rättelserna, och ⟪name|Kallas⟫ svarade att omtestet görs i slutet av ⟦capitalization|Oktober|oktober⟧. Leverantörens rapport är på engelska: ⟪foreign|"Both findings were remediated; a re-test is scheduled for the last week of October."⟫

## 3. Lösenordspolicy

Det föreslogs att lösenordets minsta längd höjs till ⟪unit|16 tecken⟫. ⟪name|Selja⟫ tyckte att förslaget var ⟪grammar_ok|bättre än⟫ ett obligatoriskt bytesintervall, som ⟪grammar_ok|avskaffades⟫ förra året. Användarna ⟦present_tense*|motsätta|motsätter⟧ sig vanligen ändringar, men denna ändring syns inte för dem eftersom ⟪grammar_ok|de flesta använder⟫ en lösenordshanterare.

Beslut: minsta längd är ⟪unit|16 tecken⟫ från ⟪unit|1.12⟫. Gamla lösenord fungerar tills användaren byter ⟦de_dem*|de|dem⟧. Ändringen ⟦present_tense*|meddela|meddelas⟧ på intranätet.

## 4. Tvåfaktorsautentisering

Alla anställda ⟪grammar_ok|som har⟫ fjärråtkomst använder redan ⟪product|Authenticator⟫. Kvar finns ⟪grammar_ok|de delade⟫ konton som används i utrustning. ⟪name|Ranta⟫ utreder om ⟦de_dem*|dem|de⟧ kan ersättas med ⟪product|YubiKey⟫.

Problemet är inte tekniskt utan administrativt: ingen äger ⟪grammar_ok|de delade⟫ kontona. Att kontona är delade är en avvikelse från policyn. Vi borde ⟦att_infinitive*|att åtgärda|åtgärda⟧ det ⟪grammar_ok|innan revisionen⟫.

## 5. Övriga ärenden

- Säkerhetsutbildningen hålls i ⟦capitalization|November|november⟧. Deltagandet är obligatoriskt för alla, ⟪grammar_ok|även cheferna⟫.
- Nästa möte ⟪unit|4.11.2026⟫. Protokollet godkänns per e-post om inga anmärkningar kommer inom ⟪unit|tre dagar⟫.

Mötet avslutades ⟪unit|kl. 13.55⟫. Protokollet ⟦supine*|skrev|skrevs⟧ av ⟪name|Ville Ranta⟫; det är ⟪grammar_ok|kortare än⟫ vanligt eftersom två ärenden ⟪grammar_ok|flyttades⟫ till nästa möte. ⟪grammar_ok|De som⟫ vill ⟪grammar_ok|kan läsa⟫ bilagorna i ⟪product|Confluence⟫.
