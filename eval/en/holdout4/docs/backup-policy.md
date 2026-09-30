# Backup policy

This policy applies to every production system that hold customer or patient data. Systems without personal data follow a lighter model.

## Principles

1. Every database are backed up at least once a day, both continuously and nightly.
2. Copies is kept in a different location from the original data. We stick to this in development environments too.
3. Restores is tested every quarter. An untested backup only looks safe.
4. Encryption keys live in Vault, where only the on-call role have access.

The policy depends on which class a system belongs to. The head of IT decides the class; the system owner may propose a different one but have to justify it in writing.

## Retention

| Target | Frequency | Retention |
|--------|-----------|-----------|
| PostgreSQL | continuous (WAL) + 01:00 | 35 days |
| File store | every 4 h | 90 days |
| Logs | daily | 1 year |
| Kubernetes configuration | every change | 2 years |

Retention are longer then the legal minimum because restore requests often arrives late. When the retention period ends, the copies are deleted automatically. Anyone who needs longer retention applies to the head of IT.

## Responsibilities

The on-call engineer check every morning that last night's copies succeeded. A failed copy are reported immediately in #incidents, and if the cause is not found within an hour the issue is escalated. The system has saved data from the last 35 days, so an single miss are rarely critical.

The database team makes sure the restore drill are run and documented. A report are written about the drill, covering duration, problems and fixes. Some drills is run without warning; the new engineers are welcome as observers.

Engineers does not take their own copies of production data to workstations. Engineers who need test data use the anonymised staging copy, which is almost always enough. We have seen few exceptions, and each of them was approved in advance.

## Deviations

If a copy are missing two days in a row, their are a security incident and the data protection officer are informed. The cause of a deviation is never purely technical; its also a process gap. The biggest risk are that nobody want to own the follow-up.

The policy are reviewed in january. A Finnish summary for vendors: Varmuuskopiot otetaan päivittäin, säilytetään 35 päivää ja palautus testataan neljännesvuosittain. The summary is translated into Swedish as well, though it is not required, and occassionally into English for auditors who's reports are public.
