# Accessibility audit: checkout flow, Q2 2026

Auditor: Elin Sandberg with Tuukka Mäkinen (screen reader testing)
Scope: web checkout, steps 1 to 4, desktop and mobile
Standard: WCAG 2.2 level AA

## Summary

Of 31 criteria tested, 22 pass, 6 fail and 3 are not applicable. The failures
cluster around the address form and the payment iframe. None block a purchase for a
sighted keyboard user; two block VoiceOver users entirely, witch makes this a
release blocker under there own policy.

## Failures

| ID | Criterion | Where | Severity |
|---|---|---|---|
| F1 | 1.3.1 Info and relationships | address form | high |
| F2 | 2.4.3 Focus order | payment iframe | high |
| F3 | 1.4.3 Contrast | disabled buttons | low |
| F4 | 3.3.1 Error identification | card number | medium |
| F5 | 2.5.8 Target size | quantity stepper | medium |
| F6 | 4.1.2 Name, role, value | country combobox | high |

### F1: address form labels

Labels are visually next to there inputs but not associated with them. A screen
reader announces "edit text" six times. Add `for` attributes, it is a one line change per field.

### F2: focus order in the payment iframe

After the card number, focus jumps to the page footer and than back into the iframe.
The cause is a `tabindex="1"` on the promo code link. Remove it; positive tabindex values are
allmost always a mistake.

### F4: card errors

The error text appears below the field but is not announced. A ARIA live region or
`aria-describedby` fixes it. Also, the message says "invalid input"; say what is wrong.
Something like "card number must have 16 digits".

### F6: country combobox

The custom dropdown is a `div` with click handlers. It has no role, no keyboard support and
no accessible name. Replace it with a native `select`; its uglier and works
everywhere, weather the user is on JAWS, TalkBack or a switch device.

## Passes worth keeping

- Skip link on every step
- Order summary is a proper table with headers
- Reduced motion is respected in the progress animation
- Zoom to 400% keeps the layout usable

## Method

Manual testing with VoiceOver on macOS 15 and iOS 18,
NVDA on Windows 11, plus axe-core 4.10 for the automated
pass. Automated tools found 2 of the 6 failures. That ratio is typical, do not rely on the scanner alone.

## Recommendations

1. Fix F1, F2 and F6 before the next release. Estimated effort 2 days.
2. Fix F4 and F5 in the following sprint.
3. F3 needs a design decision; the disbled button colour is a brand token and
   changing it effects every product. Raise it with brand rather then
   patching locally.

Re-test is booked for 2026-07-08. You're team gets the same two auditors so
findings are comparible across rounds.
