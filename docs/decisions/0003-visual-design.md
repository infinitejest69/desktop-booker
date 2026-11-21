# ADR 0003 — Visual design: slate, heather and brass key fobs

**Status:** accepted · 2026-11-21

## Context
The UI started on ui-sdk's default `web.css` look with a light tint. It worked but felt generic. The app is used for long sessions at a small family-run hotel's reception, by people of different ages, so it must be calm and very legible.

## Decision
- **Palette** (all in `src/app.css`, light and dark via `light-dark()`; every text/edge pair checked against WCAG 4.5:1 / 3:1):
  slate ink `#27323C`, paper `#FFFFFF` on a slate ground `#E9EDEF`, heather `#5E3F7A` for actions and the current page.
  Status colours share one family: moss = arriving/confirmed, estuary = in house, brass = departing/provisional, grey = done.
  Components use the `--st-*` role tokens, never their own colours.
- **Type:** Atkinson Hyperlegible Next for all reading and input (designed for legibility; its marked zero avoids 0/O mix-ups in references). Young Serif only for words that act as signage: the hotel name, page titles, the booking guest's name. Room numbers and counts use Atkinson bold, because Young Serif's old-style figures make "G01" read as "Go1". Fonts are bundled (`@fontsource`), since the app runs offline.
- **One signature element:** room numbers as brass **key fobs** (`.fob`), on the chart, the dashboard lists and the booking summary. Everything else stays quiet.
- **Avoided on purpose:** big-number stat tiles (the dashboard uses one sentence instead), all-caps labels, " · " separated meta strings, a dark admin sidebar.

## Consequences
New screens should use the tokens in `src/app.css` and the `.fob`, `.status`, `.panel` and `.link-button` classes rather than new colours.
