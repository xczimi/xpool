# Theme switcher — colour themes + dark/light mode

Status: needs-triage
Area: web

## Idea

Let users switch the app's accent colour away from the single fixed orange to a
few alternative palettes, and add a Dark / Light mode toggle.

## Motivation

The scoreboard-LED design system (#7) currently bakes in one orange accent. A
small set of selectable themes (plus dark/light) makes the pool feel
personal and improves readability for users who prefer a darker or lighter UI.

## Sketch

- A handful of preset accent palettes (orange + a few others), chosen from the
  chrome (near the language picker).
- A separate Dark / Light (and maybe "system") toggle.
- Persist the choice (localStorage) so it survives reloads; consider whether it
  should be per-device or stored on the player profile.
- Implement via CSS custom properties so the design system reads from theme
  variables rather than hardcoded colour values in `web/src/index.css`.

## Open questions

- Per-device (localStorage) or per-account (profile) preference?
- Does dark/light need a "follow system" option (`prefers-color-scheme`)?
- How many accent presets, and which colours stay on-brand with LED-X?
