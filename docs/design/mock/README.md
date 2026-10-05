# oxide-design mock

Static HTML/CSS preview of the oxide-dev-tools design system. Purpose:
**eyes-on validation of the design language** (palette, light/dark themes,
density, shell, home grid, tool-page patterns) *before* implementing it in
GPUI Kit. Once approved, this mock is the acceptance reference for the real
GUI in `crates/oxide-dev-tools-gui`.

No framework, no build step. Open `index.html` in a browser (`file://` works).

## Pages

| File | Shows |
|---|---|
| `index.html` | Shell + home: search, tool-card grid grouped by category |
| `base64.html` | Pattern A — Transform: input pane → output pane |
| `uuid.html` | Pattern B — Generator: options form → result pane |
| `email.html` | Pattern C — Validate: input → verdict badge + details |
| `settings.html` | Settings: theme (System/Light/Dark), density, switches |
| `components.html` | Every state: buttons, inputs, selection, tabs, badges, kbd, tooltip, alerts, progress, toast |
| Every page | Top-centered search above the page heading; icon in an isolated slot divided from the text, `Ctrl K` chip |

## Theme

The select in the title bar (and Settings) offers System / Light / Dark.
System follows your OS. Dark tokens live under `[data-theme="dark"]` in
`design.css`.

## Token mapping

`design.css` `:root` custom properties map 1:1 to the semantic tokens in
`../phase7-ui.md` §3.1 (`--primary`, `--background`, `--surface`, `--border`,
`--foreground`, `--muted`, `--danger`, `--success`, …). Hex values appear only
there. When the gpui-kit theme module is built, it must define the same token
names — the mock is the source of truth for values until the design review
tunes them.

## What to review (eyes-on checklist)

Derived from the design-guides review checklist, scoped to what a static mock
can show:

- [ ] Hierarchy: does the primary task (run a tool, read the result) dominate each page?
- [ ] Search: very top center of every page, above the page title; icon isolated from the text by a divider
- [ ] Resizing: shrink the browser toward the ~900×640 minimum — panes adapt, regions scroll, nothing clips
- [ ] Color in both themes: orange accent, contrast of `primary`/`muted` on `surface`, danger vs brand distinction
- [ ] Density: comfortable but not airy; medium default
- [ ] Minimalism: one accent, borders over shadows, no gradients, mono only for data
- [ ] Card grid: two-line cards (name + one-line description), scannable at ~1200×800
- [ ] Tool pages: top-centered content — A/B patterns capped ~1060px and filling the window height, C capped 560px; same pane headers/actions/paddings across patterns
- [ ] States: hover, focus ring, selected, disabled, loading, validation, destructive
- [ ] Lexicon: noun destinations, verb commands, sentence case, `Export…` ellipsis
- [ ] Sidebar: category grouping matches the README feature table, groups expand/collapse on click, active state visible

## What this mock cannot validate

- Real gpui-kit component availability and APIs
- The theme system's actual behavior and fonts
- Keyboard focus flow, Escape dismissal, resize, platform chrome

Those are verified in the gpui-kit implementation spike (Phase 7.4), checked
against this mock.
