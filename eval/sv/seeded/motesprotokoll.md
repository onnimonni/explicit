# Protokoll: gemensamt möte för IT-förvaltningen och de svenskspråkiga tjänsterna

Tid: onsdagen den 9 september 2026 kl. 13–14.30
Plats: Sinebrychoffs mötesrum, 4 våningen
Närvarande: Tove Jansson (ordf.), Bo Carpelan, Kjell Westö, Eino Leino (sekr.)

## 1. Mötet öppnas

Ordföranden öppnade mötet och konstaterade att det var beslutfört. Föregående mötes protokoll godkändes utan ändringar.

## 2. Läget för det svenskspråkiga användargränssnittet

Carpelan redogjorde för översättningarbetet. 82 % av användar gränssnittet är översatt. Det som saknas gäller främst felmeddelanden och inställningssidan, och Svenskspråkiga kunder har gett respons som samlats i Jira.

Westö påpekade att en del översättningar är maskinöversatta och innehåller term fel. Han läste upp ett exempel ur responsen:

> "Painike 'Peruuta' on käännetty muotoon 'Avbryt', mikä on oikein, mutta 'Keskeytä' on käännetty myös 'Avbryt'."

Det beslöts att en språkexpert granskar översättningarna före publiceringen. Institutet för de inhemska språken har lovat kommentera termlistan inom två veckor.

## 3. Teknisk lösning för språkvalet

Språkvalet sparas i dag i en kaka, vilket inte fungerar om användaren har blockerat kakor. Det föreslogs att språket sparas i användarens profil och skickas i headern `Accept-Language`.

```typescript
const lang = user.profile.language ?? negotiate(req.headers["accept-language"]);
res.setHeader("Content-Language", lang);
```

Jansson frågade om språket kan härledas ur modersmåls uppgiften hos Myndigheten för digitalisering och befolkningsdata. Det konstaterades att uppgiften bara får användas med kundens samtycke. Saken utreeds med dataskyddsombudet.

## 4. Nordiskt samarbete

Westö berättade om besöket hos Region Stockholm. I Sverige har motsvarande integration gjorts mot 1177. De är intresserade av våra erfarenheter av Kanta-anslutningen. En gemensam workshop ordnas i oktober i Stockholm; de svenska kollegerna står för arrangemangen.

Resekostnaderna täcks av EU:s Interreg-finansiering. Reseräkningarna lämnas in inom en månad efter resan.

## 5. Övriga ärenden

- Leino påminde om att utvecklingsmiljöns lösenord byts på fredag.
- Nästa möte hålls den 7 oktober 2026, ifall översättningsarbetet är klart tills dess.

Ordföranden avslutade mötet kl. 14.25. Innan protokollet publiceras har deltagarna tre dagar på sig att kommentera den.

Yhteenveto suomeksi: käyttöliittymän käännöksestä on valmiina 82 %, kieliasiantuntija tarkastaa termit ennen julkaisua, ja seuraava kokous pidetään 7. lokakuuta.
