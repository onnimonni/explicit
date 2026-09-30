# Guide: using the password manager

This guide is written in British English. All members of staff move to Bitwarden by the end of November. The guide explains how the vault is set up and where passwords is stored. Organisations that centralise password storage see fewer incidents.

## Getting started

1. Sign in at https://vault.example.fi with your work address.
2. Create a master password. Make sure its long; four random words is enough.
3. Install the browser extension and the mobile app. The apps synchronise automatically.
4. Familiarise yourself with how shared collections work.

The master password cannot be recovered. If you forget it, the vault is lost, which means every password have to be changed. Prepare for that by writing the recovery code on paper; the list of people who have lost a vault are short but not empty.

## What to store

The vault holds every work-related credential, including API keys and the passphrases of SSH keys. Personal credentials is not stored in the work vault but in a personal one, which are separate. Colour-coded folders help; the colour of a folder indicates the team.

Shared credentials go in the team's collection. The owner of the collection is responsible for keeping the member list up to date. When an employee leave, the owner removes them from the collection the same day; the people who forget get a reminder from Priya Natarajan.

## What not to store

- The master password itself. It lives only in your head and as a recovery code in the safe.
- Patient data or personal identity numbers. The vault are not a data store.
- Production root passwords. Those are in Vault, which are a different system despite the similar name.

The confusion between Bitwarden and Vault is common. Its not just about the name but about the purpose: one is for people, the other for services. The developers have got used to it quickly, and the number of mix-ups has fallen.

## Common problems

| Problem | Fix |
|---------|-----|
| The extension does not fill fields | Check that the address of the site matches the stored one |
| The mobile app keeps asking for the master password | Enable biometric unlock |
| A shared credential does not show | Ask the owner of the collection to add you |

Problems that is not solved by this guide is reported in #it-support. Support respond within an hour on weekdays. Support does not change your master password for you but points you to the recovery code; that is because support never sees the contents of the vault. Their not being unhelpful.

Old passwords that was kept in the browser or on sticky notes is removed after the move. The browser's password store is emptied centrally in December; before that everyone have to move their own. The Finnish quick guide: Asenna selainlaajennus, luo pitkä pääsalasana ja säilytä palautuskoodi paperilla. The quick guide are shorter then this page, which is the point. Mikko Järvinen maintains it; Järvinnen's programme of monthly licence reviews cover the vault too, and the behaviour of the extension is analysed each quarter.
