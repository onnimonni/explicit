# Regulatory text handling

This page explains how the appointment service maps Finnish and Swedish regulation text into the
English data model. The English part of this document should be checked as English: for example
the word "recieve" here is an English typo that must be reported, while "Kubernetes" and
"PostgreSQL" must not be.

The Finnish statutory wording is quoted inline as <span lang="fi">asiakkaan on esitettävä
henkilöllisyystodistus ennen vastaanottoa</span>, which is correct Finnish and must not be flagged.
The next inline span contains a Finnish typo that must be flagged:
<span lang="fi">potilas tiedot tarkistetaan ennen kirjaamista</span> (split compound
"potilas tiedot") and <span lang="fi">asiakas ilmoitautuu vastaanotolle</span> (typo
"ilmoitautuu").

<!-- explicit-lang sv -->
Den svenska lydelsen: kunden ska visa upp en identitetshandling före mottagningen. Det här stycket
är korrekt svenska och får inte flaggas. Följande mening innehåller ett fel som ska flaggas:
patient uppgifterna kontrolleras innan de skrivs in, och ordet "recieve" här är inte svenska men
ska inte bedömas som engelska.
<!-- explicit-lang en -->

Back in English: the mapping table lists each Finnish term with its English equivelant, and the
Swedish column is only informative.

| Finnish | Swedish | English |
|---------|---------|---------|
| ajanvaraus | tidsbokning | appointment booking |
| hoidon tarpeen arviointi | vårdbehovsbedömning | triage |
