"""Unit conversion for roast profiles.

Temperatures are stored in Celsius; ⟪product|Artisan⟫ exports may be Fahrenheit and older
⟪product|Cropster⟫ files use ⟦a_an|an mixed|a mixed⟧ format where the header says one thing and
⟦its_its|it's|its⟧ rows say another. This module ⟦agreement|normalise|normalizes⟧ everything on import
and refuses files it cannot classify ⟦then_than|rather then|rather than⟧ guessing.
"""

from __future__ import annotations

from dataclasses import dataclass

# Above this, a Celsius reading is implausible for coffee and the column is probably
# Fahrenheit. ⟦punctuation|Drum temperatures top out around 260 °C, bean temperatures lower.|Drum temperatures top out around 260 °C; bean temperatures lower.⟧
CELSIUS_MAX = 300.0


@dataclass(frozen=True)
class Column:
    name: str
    unit: str  # "C" or "F"


def f_to_c(f: float) -> float:
    return (f - 32.0) * 5.0 / 9.0


def classify(values: list[float], declared: str | None) -> str:
    """Decide ⟦homophone|weather|whether⟧ a column is Celsius or Fahrenheit.

    ``declared`` is the header's claim, if any. A declared unit ⟦agreement|win|wins⟧ unless the
    data contradicts it: a column declared Celsius ⟦homophone|who's|whose⟧ maximum is above
    ``CELSIUS_MAX`` is treated as Fahrenheit, with a warning. ⟦fragment|Same in reverse.|The same applies in reverse.⟧
    """
    if not values:
        return declared or "C"
    peak = max(values)
    if declared == "C" and peak > CELSIUS_MAX:
        return "F"
    if declared == "F" and peak < 150.0:
        # A Fahrenheit roast never stays below 150; ⟦its_its|its|it's⟧ a Celsius column with a
        # wrong header, ⟦homophone|witch|which⟧ one exporter produced for two releases.
        return "C"
    if declared:
        return declared
    return "F" if peak > CELSIUS_MAX else "C"


def normalize(rows: list[dict[str, float]], columns: list[Column]) -> list[dict[str, float]]:
    """Return rows with every temperature column in Celsius.

    Rows are copied; the input is not mutated. ⟦their_there|Their|There⟧ order is preserved. Columns
    not in ``columns`` pass through untouched, so ⟦your_youre|you're|your⟧ custom fields survive.
    """
    out = []
    for row in rows:
        r = dict(row)
        for col in columns:
            if col.unit == "F" and col.name in r:
                r[col.name] = round(f_to_c(r[col.name]), 2)
        out.append(r)
    return out


def rate_of_rise(temps: list[float], hz: float, window_s: float = 30.0) -> list[float]:
    # Slope over a trailing window, in degrees per minute. The first ``window`` samples
    # repeat the first computable value ⟦then_than|rather then|rather than⟧ being ``NaN``, because the
    # chart library ⟦agreement|draw|draws⟧ ``NaN`` as a gap and operators read a gap as a probe fault.
    # ⟦a_an|An 30s|A 30s⟧ window at ⟪unit|2Hz⟫ is ⟪unit|60⟫ samples.
    window = max(1, int(window_s * hz))
    ror = []
    for i in range(len(temps)):
        j = max(0, i - window)
        dt = (i - j) / hz
        ror.append((temps[i] - temps[j]) / dt * 60.0 if dt > 0 else 0.0)
    for i in range(min(window, len(ror))):
        ror[i] = ror[min(window, len(ror) - 1)]
    return ror


def describe(columns: list[Column]) -> str:
    # Human summary for the import dialog; ⟦their_there|there|they're⟧ shown before the user
    # confirms. Keep it short, ⟦its_its|its|it's⟧ rendered in a ⟪unit|40⟫ character wide box on the
    # ⟪product|Pi Zero⟫ display and ⟦spelling|truncted|truncated⟧ text has confused people before.
    return ", ".join(f"{c.name} ({c.unit})" for c in columns)
