# Language picker — replace the dropdown

Status: needs-triage
Area: web

## Idea

Replace the language selector dropdown with a more direct control.

## Motivation

There are only two languages (English + Hungarian). A `<select>` dropdown is
heavyweight for a binary choice — it hides the options behind a click and reads
as a generic form control rather than part of the chrome.

## Sketch

- A segmented toggle / pair of inline buttons (`EN | HU`) rendered directly in
  the chrome, with the active language highlighted.
- Keep it accessible (keyboard + screen-reader labels) and i18n-driven from
  `web/src/i18n/strings.ts`.
- Should sit naturally next to the (future) theme switcher.

## Open questions

- Flag icons, two-letter codes, or full language names on the toggle?
- Does this generalise if a third language is ever added, or is two-only fine?
