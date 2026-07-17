# Held-back major dependency bumps

All holds from the 2026-07-17 sweep are cleared (staged migration complete).

## Done

| Crate | From | To | Notes |
| --- | --- | --- | --- |
| `ratatui` | 0.29.0 | 0.30.2 | Root fix; modular facade (`ratatui-core` / widgets / crossterm) |
| `crossterm` | 0.28.1 | 0.29.0 | Lockstep with ratatui 0.30 default backend |
| `tui-markdown` | 0.3.6 | 0.3.8 | Aligns via `ansi-to-tui` on ratatui-core types |
| `ratatui-image` | 1.0.1 | 11.0.6 | `Protocol` is now an enum; `Picker::from_query_stdio` / `Size`; no chafa (`default-features = false`, `image-defaults`) |
| `tui-textarea` | 0.7.0 | **tui-textarea-2 0.12.1** | Upstream stuck on ratatui 0.29; package rename keeps `tui_textarea` import path |
| `reqwest` | 0.12.28 | 0.13.4 | Independent; default TLS is rustls (aws-lc) |

## Verify

```bash
cargo tree -i ratatui          # single 0.30.x
cargo tree -i crossterm        # single 0.29.x
cargo check
```
