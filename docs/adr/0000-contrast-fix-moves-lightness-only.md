# 0000. The contrast fix moves only lightness, against the background and the surface

**Number:** next free.

**Status:** Accepted, 2026-10-26. Phase 3 work pulled forward, part 3.4.

## Context

Task 3.4 asks theme sync to give a text and background pair that fails WCAG 2.2 AA "the nearest passing shade". The plan does not say what nearest means, which colours count as text, or which backgrounds they sit on.

## Decision

- A failing colour keeps its HSL hue and saturation; only its lightness moves. The search steps outward both ways from the old lightness in steps of 0.1%, finer than one step of an 8-bit channel, and takes the first shade that passes. At equal distance the darker shade wins.
- `colors.text`, `colors.muted` and `colors.link` need 4.5 to 1 (success criterion 1.4.3). `colors.accent` marks buttons and highlights and needs 3 to 1 (1.4.11). `colors.border` is decoration and is not checked.
- Each colour must pass on both `colors.background` and `colors.surface`, because templates put text on cards and code blocks as well as on the page.
- When no shade passes on both, the colour gets the shade with the most contrast, and the message tells the person to change the background or the surface.
- Ratios in messages are rounded down to one decimal, so a failing 4.47 never reads as 4.5.

## Consequences

- The brand hue survives: a pale amber link becomes a darker amber, not a different colour.
- Lightness is not perceptually even, so another colour space such as OKLCH could find a shade that looks closer. HSL is enough for the done line and needs no new dependency.
