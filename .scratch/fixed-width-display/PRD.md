# Revisit the fixed-width display

Status: deferred — dropped this round (2026-06-27, Peter's call)

## Resolved decisions (2026-06-27 grill)

- **DROPPED this round.** When asked directly (rather than dropped for vagueness),
  Peter chose to defer — nothing concrete itches yet. Revisit in a later round.
- If revisited: first do an audit (no code) of each page at mobile / tablet / wide /
  ultrawide, list concrete pain points, then decide global vs per-view widths.
Area: web

## Idea

Re-evaluate the app's fixed-width layout container — whether the current max
width is right across screen sizes, and whether some views should go fluid /
wider.

## Motivation

The SPA renders inside a fixed-width shell. Wide data views (All Tips grid,
standings tables, schedule) can feel cramped on large screens while the same
fixed width may not adapt well on small/mobile screens.

## Sketch

- Audit each page against narrow (mobile), tablet, and wide-desktop widths.
- Decide per-view: keep the centred fixed column for reading-heavy pages, but
  allow data-dense tables/grids to use more horizontal space.
- Check the LED scoreboard design system's spacing/breakpoints still hold.

## Open questions

- One global container width, or per-view widths?
- Is there a mobile layout story yet, or is this desktop-first for now?
- Which specific pages feel worst today? (worth listing before building)
