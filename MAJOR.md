# Held-back major dependency bumps

Written after the 2026-07-17 dependency sweep. `ratatui` is the root of everything below — it's
pinned at 0.29.0, and every TUI-adjacent crate that's moved to the newer `ratatui-core`-split
architecture breaks against it. This isn't hypothetical: **three separate crates already hit this
exact wall today** (`ratatui-image`, `tui-markdown`, and now Renovate's proposed `tui-textarea`
bump), each independently pulling in a duplicate `ratatui-core`/newer-`ratatui` copy that doesn't
type-match this crate's own pinned `ratatui` 0.29 types (`Rect`, `Text`, `Widget`, etc. become two
distinct, non-interchangeable types). The real fix is bumping `ratatui` itself — everything else
is a symptom.

## The actual fix: ratatui 0.29 → 0.30

Not done today — this needs a deliberate pass, not a side-effect bump. Ratatui 0.30 split the
crate into `ratatui-core` + `ratatui-widgets` + `ratatui-crossterm` re-exported through the
`ratatui` facade crate; most consumer code keeps working unchanged if it only uses the
`ratatui::prelude`/top-level re-exports, but this project has hand-written TUI code across
`src/tui/` that imports concrete paths (e.g. `Rect`, `Paragraph`, `Widget`) — those need to be
verified against 0.30's re-export surface, not just recompiled and hoped for. Check the
[ratatui 0.30 changelog](https://github.com/ratatui/ratatui/releases) for the full migration
notes before starting.

Once `ratatui` itself is on 0.30, re-attempt the three holds below — they should very likely just
work at their already-available latest versions (all three were held specifically *because* of
the version skew against 0.29, not for any unrelated reason):

## Held pending the ratatui bump

| Crate | Held at | Available | Confirmed break |
| --- | --- | --- | --- |
| `ratatui-image` | 1.0.1 | 1.0.5 (or 11.0.6, a separate later major renumbering) | Pulls in `ratatui-core` + duplicate `ratatui` 0.30.2 → `Rect`/`Text`/`Widget` type mismatch against this crate's pinned 0.29 |
| `tui-markdown` | 0.3.6 | 0.3.8 | Same issue, via its `ansi-to-tui` dependency |
| `tui-textarea` | 7.0.0 (untouched — Renovate proposed 8.0.1, not yet attempted locally) | 8.0.1 | Renovate's PR #2 CI run shows `E0308`/`E0277` on `&tui_textarea::TextArea<'_>: ratatui::prelude::Widget` — same root cause, confirmed by CI, not yet locally reproduced |

## Also held (unrelated to the ratatui cluster)

- **crossterm 0.28.1 → 0.29.0** — `ratatui`'s own crossterm backend version needs to move in
  lockstep with whichever `ratatui` version you land on; check what `ratatui` 0.30 actually
  requires before picking a crossterm version independently.
- **reqwest 0.12.28 → 0.13.4** — same TLS feature rename (`rustls-tls` → `rustls`) hit in
  `stash-mcp`/`plex-rs`/`tei-proxy` today. Independent of the ratatui cluster; can be done
  separately whenever convenient.

## Suggested order

1. **ratatui 0.29 → 0.30** first, as its own dedicated PR — expect to touch every file under
   `src/tui/`. Budget real review time; this is not a mechanical bump.
2. Re-run `cargo upgrade --incompatible` afterward — `ratatui-image`, `tui-markdown`, and
   `tui-textarea` should resolve to their latest versions cleanly once the type mismatch is gone.
   Verify rather than assume.
3. **reqwest** whenever, independently — small, mechanical, per the note in the other three
   repos' `MAJOR.md`.

## Until then

Renovate will keep proposing `tui-textarea` (and anything else in this cluster) as "non-major" —
it doesn't know these break, only that they're semver-compatible per its own classification.
Either close each new PR as it appears, or add a `packageRules` entry to `renovate.json`
disabling major-version PRs for this repo (matching the pattern already used in
`the-bannered-mare`'s `renovate.json`) if you'd rather stop seeing them.
