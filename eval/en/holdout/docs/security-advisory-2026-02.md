# Security advisory: path traversal in template includes

Identifier: GHSA-q4m7-xx2p-9h3c
Severity: high (CVSS 7.5)
Affected: tplr 0.9.0 through 1.3.2
Fixed in: 1.3.3, 1.2.8

## Summary

The `{% include %}` tag resolved paths relative to the template root but did not reject
`..` segments after normalisation. A template author could include
any file readable by the process, such as /etc/passwd or .env files. Template
authors are usually trusted, but several users render templates supplied by there
customers, who's trust level is lower.

## Impact

Disclosure of local files. No write primitive and no code execution. Deployments where template
authors are also administrators is not meaningfully affected.

## Timeline

| Date | Event |
|---|---|
| 2026-02-03 | Report received from Vesa Heikkilä |
| 2026-02-04 | Confirmed; fix drafted |
| 2026-02-10 | Releases published; advisory embargoed 7 days |
| 2026-02-17 | Public |

## Fix

Includes are now resolved with `Path::components()` and any `ParentDir` component
rejcted before joining. Symlinks pointing outside the root are than
detected by canonicalizing the result and checking the prefix. Both checks are needed, the second alone races with a symlink swap.

## Workarounds

If you cannot upgrade, set `Loader::sandboxed(true)`, available since 0.9, which
disables includes entirely. Its a blunt tool but a effective one.

## Credit

Thanks to Vesa Heikkilä for the report and the patient back and forth on the symlink
case. A textbook disclosure.
