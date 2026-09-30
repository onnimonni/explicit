# Security advisory: path traversal in template includes

Identifier: ⟪acronym|GHSA⟫-q4m7-xx2p-9h3c
Severity: high (⟪acronym|CVSS⟫ ⟪unit|7.5⟫)
Affected: ⟪product|tplr⟫ ⟪version|0.9.0⟫ through ⟪version|1.3.2⟫
Fixed in: ⟪version|1.3.3⟫, ⟪version|1.2.8⟫

## Summary

The ⟪code|`{% include %}`⟫ tag resolved paths relative to the template root but did not reject
⟪code|`..`⟫ segments after ⟦british|normalisation|normalization⟧. A template author could include
any file readable by the process, such as ⟪path|/etc/passwd⟫ or ⟪path|.env⟫ files. Template
authors are usually trusted, but several users render templates supplied by ⟦their_there|there|their⟧
customers, ⟦homophone|who's|whose⟧ trust level is lower.

## Impact

Disclosure of local files. No write primitive and no code execution. Deployments where template
authors are also administrators ⟦agreement|is|are⟧ not meaningfully affected.

## Timeline

| Date | Event |
|---|---|
| ⟪unit|2026-02-03⟫ | ⟪table|Report received⟫ from ⟪name|Vesa Heikkilä⟫ |
| ⟪unit|2026-02-04⟫ | ⟪table|Confirmed; fix drafted⟫ |
| ⟪unit|2026-02-10⟫ | ⟪table|Releases published; advisory embargoed 7 days⟫ |
| ⟪unit|2026-02-17⟫ | ⟪table|Public⟫ |

## Fix

Includes are now resolved with ⟪code|`Path::components()`⟫ and any ⟪code|`ParentDir`⟫ component
⟦spelling|rejcted|rejected⟧ before joining. Symlinks pointing outside the root are ⟦then_than|than|then⟧
detected by canonicalizing the result and checking the prefix. ⟦punctuation|Both checks are needed, the second alone races with a symlink swap.|Both checks are needed; the second alone races with a symlink swap.⟧

## Workarounds

If you cannot upgrade, set ⟪code|`Loader::sandboxed(true)`⟫, available since ⟪version|0.9⟫, which
disables includes entirely. ⟦its_its|Its|It's⟧ a blunt tool but ⟦a_an|a effective|an effective⟧ one.

## Credit

Thanks to ⟪name|Vesa Heikkilä⟫ for the report and the patient back and forth on the symlink
case. ⟦fragment|A textbook disclosure.|It was a textbook disclosure.⟧
