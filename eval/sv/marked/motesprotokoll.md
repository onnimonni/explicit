# Protokoll: gemensamt möte för IT-förvaltningen och de svenskspråkiga tjänsterna

Tid: onsdagen den 9 september 2026 ⟪unit|kl. 13–14.30⟫
Plats: ⟪name|Sinebrychoffs⟫ mötesrum, ⟪ordinal|4 våningen⟫
Närvarande: ⟪name|Tove Jansson⟫ (ordf.), ⟪name|Bo Carpelan⟫, ⟪name|Kjell Westö⟫, ⟪name|Eino Leino⟫ (sekr.)

## 1. Mötet öppnas

Ordföranden öppnade mötet och konstaterade att det var beslutfört. Föregående mötes protokoll godkändes utan ändringar.

## 2. Läget för det svenskspråkiga användargränssnittet

⟪name|Carpelan⟫ redogjorde för ⟦compound_link|översättningarbetet|översättningsarbetet⟧. ⟪unit|82 %⟫ av ⟦sarskrivning|användar gränssnittet|användargränssnittet⟧ är översatt. Det som saknas gäller främst felmeddelanden och inställningssidan, och ⟦capitalization|Svenskspråkiga|svenskspråkiga⟧ kunder har gett respons som samlats i ⟪product|Jira⟫.

⟪name|Westö⟫ påpekade att en del översättningar är maskinöversatta och innehåller ⟦sarskrivning|term fel|termfel⟧. Han läste upp ett exempel ur responsen:

> ⟪foreign|"Painike 'Peruuta' on käännetty muotoon 'Avbryt', mikä on oikein, mutta 'Keskeytä' on käännetty myös 'Avbryt'."⟫

Det beslöts att en språkexpert granskar översättningarna före publiceringen. ⟪name|Institutet för de inhemska språken⟫ har lovat kommentera termlistan inom ⟪unit|två veckor⟫.

## 3. Teknisk lösning för språkvalet

Språkvalet sparas i dag i en kaka, vilket inte fungerar om användaren har blockerat kakor. Det föreslogs att språket sparas i användarens profil och skickas i headern ⟪identifier|`Accept-Language`⟫.

```typescript
const lang = user.profile.language ?? negotiate(req.headers["accept-language"]);
res.setHeader("Content-Language", lang);
```

⟪name|Jansson⟫ frågade om språket kan härledas ur ⟦sarskrivning|modersmåls uppgiften|modersmålsuppgiften⟧ hos ⟪name|Myndigheten för digitalisering och befolkningsdata⟫. Det konstaterades att uppgiften bara får användas med kundens samtycke. Saken ⟦typo|utreeds|utreds⟧ med dataskyddsombudet.

## 4. Nordiskt samarbete

⟪name|Westö⟫ berättade om besöket hos ⟪name|Region Stockholm⟫. I Sverige har motsvarande integration gjorts mot ⟪name|1177⟫. De är intresserade av våra erfarenheter av ⟪fi_sv|Kanta⟫-anslutningen. En gemensam workshop ordnas i oktober i Stockholm; de svenska kollegerna står för arrangemangen.

Resekostnaderna täcks av ⟪acronym|EU⟫:s ⟪product|Interreg⟫-finansiering. Reseräkningarna lämnas in inom ⟪unit|en månad⟫ efter resan.

## 5. Övriga ärenden

- ⟪name|Leino⟫ påminde om att utvecklingsmiljöns lösenord byts på fredag.
- Nästa möte hålls den 7 oktober 2026, ifall översättningsarbetet är klart tills dess.

Ordföranden avslutade mötet ⟪unit|kl. 14.25⟫. Innan protokollet publiceras har deltagarna ⟪unit|tre dagar⟫ på sig att kommentera ⟦de_dem_gender*|den|det⟧.

⟪foreign|Yhteenveto suomeksi: käyttöliittymän käännöksestä on valmiina 82 %, kieliasiantuntija tarkastaa termit ennen julkaisua, ja seuraava kokous pidetään 7. lokakuuta.⟫
