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

- **Navigation:**
    - `Ctrl+A`: Switch to the **Models** tab.
    - `Ctrl+D`: Switch to the **Chat** tab.
    - `Tab`: Cycle focus between UI panes (e.g., List vs. Info, Input vs. Sessions).
    - `Ctrl+q`: Quit the application.

### Models Tab

- `j` / `k` (or Arrows): Navigate the model list.
- `Enter`: Select model and start a new chat.
- `p`: Pull a new model (opens a popup to enter model name, e.g., `llama3`).
- `d`: Delete the selected model.

### Chat Tab

- **Pane Navigation:**
    - `Ctrl+Left` / `Ctrl+Right`: Switch focus between **Session List** and **Input**.
    - The active pane is highlighted with a **Thick Yellow Border**.

- **Sessions (when focused):**
    - `j` / `k` (or Arrows): Navigate session history.
    - `Enter`: Load selected session (and switch to its model).
    - `d`: Delete selected session.
    - `Ctrl+n`: Start a new session.

- **Input (when focused):**
    - Type your message and press `Enter` to send.
    - `Ctrl+n`: Start a new session.
    - `PageUp` / `PageDown`: Scroll the chat history.

## Configuration

Data and logs are stored in your system's standard configuration directories (XDG on Linux).

- **Config/Logs:** `~/.config/vicuna/`
- **Database:** `~/.config/vicuna/vicuna.db`

## License

MIT — see [LICENSE](LICENSE). Do whatever you want with it, just don't blame me.
