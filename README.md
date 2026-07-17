<div align="center">

# Vicuna

**Vicuna** is a minimalist terminal-based client for [Ollama](https://ollama.com/), built in **Rust**.

[<img src="asset/vicuna-logo.png" width="400" alt="Vicuna Logo" />](https://github.com/delfianto/vicuna)

</div>

> ⚠️ **Important, Extremely Serious Disclaimer™**
>
> - This app does not come with _any_ warranty. If your GPU melts because you tried to run a 320B model on a toaster, that’s between you and your toaster.
> - This project was coded with a frankly irresponsible amount of AI assistance. Call it vibecode, smutcode, AI slop, AI gruel — I genuinely do not care.
> - This is an app for shits and giggles. If that fact causes you emotional distress, please close this tab, touch some grass, and re-evaluate your life choices.

## Features

- **🚀 Lightweight (kinda):** Built with Rust and `ratatui`.
- **💾 Local Database:** SQLite backend (via LibSQL) stores sessions and message history.
- **🤖 Model Management:** View, pull, inspect, and delete models directly from the TUI.
- **📝 Markdown Support:** Rich rendering of assistant responses including code blocks and formatting.
- **🔄 Streaming:** Real-time token streaming for immediate feedback.
- **🧹 Auto-Cleanup:** Deleting a model automatically removes its associated chat history.

## Installation

### Prerequisites

- [Ollama](https://ollama.com/) running locally (default: `http://localhost:11434`).
- Rust toolchain (cargo).

### Build from Source

```bash
git clone https://github.com/delfianto/vicuna.git
cd vicuna
cargo run --release
```

## Usage

Opens on the **Chat** tab (chat-first). Bottom bars are contextual: status shows
active tab / model / stream state; help only lists keys for the focused pane.

- **Navigation:**
    - `F2`: Open the **Models** library.
    - `F1`: Back to **Chat**.
    - `Tab`: Cycle focus between panes (sessions → conversation → composer).
    - `Esc`: Back — close overlay / close details / leave library → chat / leave insert → conversation → sessions.
    - `Ctrl+C` twice: Quit (first press cancels a stream if one is running).

### Models Tab (library · `F2`)

- `j` / `k`: Navigate the model list.
- `F3` / `i`: **Inspect** selected model (right pane: summary, parameters, template, modelfile, license). Toggle again or `Esc` to close.
- `Tab`: Focus list ↔ inspect (opens inspect if closed).
- `Enter`: Use model and start a new chat.
- `p`: Pull a model · `d`: Delete (confirm) · `s`: Sort.

### Chat Tab

Focus badge: **LIST** (sessions) · **CHAT** (transcript) · **INS** (composer).

- **Insert:** type · `Enter` send · `Esc`/`Tab` leave · `F2` library · `Ctrl+r` regen · `Ctrl+C` cancel / double-quit.
- **Chat (transcript):** `↑`/`↓` or `j`/`k` scroll · `Tab` cycle · `i` insert · `G` bottom.
- **Sessions:** `j`/`k` nav · `Enter` open · `F2` library · `m` model picker · `d` delete (confirm).
- Mouse: wheel scrolls under cursor; click pane/row to focus/select.
- **Mouse:** wheel scrolls the panel under the cursor; click a session/model row to select; click composer / sessions / conversation to focus.

## Configuration

Data and logs are stored in your system's standard configuration directories (XDG on Linux).

- **Config/Logs:** `~/.config/vicuna/`
- **Database:** `~/.config/vicuna/vicuna.db`

## License

MIT — see [LICENSE](LICENSE). Do whatever you want with it, just don't blame me.
