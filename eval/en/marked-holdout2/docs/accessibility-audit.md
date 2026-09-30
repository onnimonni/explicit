# Accessibility audit: checkout flow, Q2 2026

Auditor: ⟪name|Elin Sandberg⟫ with ⟪name|Tuukka Mäkinen⟫ (screen reader testing)
Scope: web checkout, steps 1 to 4, desktop and mobile
Standard: ⟪acronym|WCAG⟫ ⟪version|2.2⟫ level ⟪table|AA⟫

## Summary

Of ⟪unit|31⟫ criteria tested, ⟪unit|22⟫ pass, ⟪unit|6⟫ fail and ⟪unit|3⟫ are not applicable. The failures
cluster around the address form and the payment iframe. None ⟦agreement|block|blocks⟧ a purchase for a
sighted keyboard user; two block ⟪product|VoiceOver⟫ users entirely, ⟦homophone|witch|which⟧ makes this a
release blocker under ⟦their_there|there|their⟧ own policy.

## Failures

| ID | Criterion | Where | Severity |
|---|---|---|---|
| ⟪table|F1⟫ | ⟪table|1.3.1 Info and relationships⟫ | ⟪table|address form⟫ | ⟪table|high⟫ |
| ⟪table|F2⟫ | ⟪table|2.4.3 Focus order⟫ | ⟪table|payment iframe⟫ | ⟪table|high⟫ |
| ⟪table|F3⟫ | ⟪table|1.4.3 Contrast⟫ | ⟪table|disabled buttons⟫ | ⟪table|low⟫ |
| ⟪table|F4⟫ | ⟪table|3.3.1 Error identification⟫ | ⟪table|card number⟫ | ⟪table|medium⟫ |
| ⟪table|F5⟫ | ⟪table|2.5.8 Target size⟫ | ⟪table|quantity stepper⟫ | ⟪table|medium⟫ |
| ⟪table|F6⟫ | ⟪table|4.1.2 Name, role, value⟫ | ⟪table|country combobox⟫ | ⟪table|high⟫ |

### F1: address form labels

Labels are visually next to ⟦their_there|there|their⟧ inputs but not associated with them. A screen
reader announces "edit text" six times. ⟦punctuation|Add `for` attributes, it is a one line change per field.|Add `for` attributes; it is a one line change per field.⟧

### F2: focus order in the payment iframe

After the card number, focus jumps to the page footer and ⟦then_than|than|then⟧ back into the iframe.
The cause is a ⟪code|`tabindex="1"`⟫ on the promo code link. Remove it; positive tabindex values are
⟦spelling_1edit|allmost|almost⟧ always a mistake.

### F4: card errors

The error text appears below the field but is not announced. ⟦a_an|A ARIA|An ARIA⟧ live region or
⟪code|`aria-describedby`⟫ fixes it. Also, the message says "invalid input"; say what is wrong.
⟦fragment|Something like "card number must have 16 digits".|Use something like "card number must have 16 digits".⟧

### F6: country combobox

The custom dropdown is a ⟪code|`div`⟫ with click handlers. It has no role, no keyboard support and
no ⟪derived|accessible⟫ name. Replace it with a native ⟪code|`select`⟫; ⟦its_its|its|it's⟧ uglier and works
everywhere, ⟦homophone|weather|whether⟧ the user is on ⟪product|JAWS⟫, ⟪product|TalkBack⟫ or a switch device.

## Passes worth keeping

- ⟪list|Skip link on every step⟫
- ⟪list|Order summary is a proper table with headers⟫
- ⟪list|Reduced motion is respected in the progress animation⟫
- ⟪list|Zoom to 400% keeps the layout usable⟫

## Method

Manual testing with ⟪product|VoiceOver⟫ on ⟪product|macOS⟫ ⟪version|15⟫ and ⟪product|iOS⟫ ⟪version|18⟫,
⟪product|NVDA⟫ on ⟪product|Windows⟫ ⟪version|11⟫, plus ⟪product|axe-core⟫ ⟪version|4.10⟫ for the automated
pass. Automated tools found ⟪unit|2⟫ of the ⟪unit|6⟫ failures. ⟦punctuation|That ratio is typical, do not rely on the scanner alone.|That ratio is typical; do not rely on the scanner alone.⟧

## Recommendations

1. Fix F1, F2 and F6 before the next release. Estimated effort ⟪unit|2 days⟫.
2. Fix F4 and F5 in the following sprint.
3. F3 needs a design decision; the ⟦spelling|disbled|disabled⟧ button ⟦british|colour|color⟧ is a brand token and
   changing it ⟦homophone|effects|affects⟧ every product. Raise it with brand ⟦then_than|rather then|rather than⟧
   patching locally.

Re-test is booked for ⟪unit|2026-07-08⟫. ⟦your_youre|You're|Your⟧ team gets the same two auditors so
findings are ⟦spelling_1edit|comparible|comparable⟧ across rounds.
