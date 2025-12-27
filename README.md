# Vicuna

**Vicuna** is a high-performance, feature-rich terminal-based client for [Ollama](https://ollama.com/), built in **Rust**. It aims to provide a robust, "OpenWebUI-like" experience directly in your terminal, focusing on speed, type safety, and efficient resource management.

## Features

- **🚀 High Performance:** Built with Rust and `ratatui` for 60 FPS responsiveness.
- **💾 Local Database:** SQLite backend (via LibSQL) stores sessions and message history.
- **🧠 Model Management:** View, pull, and delete models directly from the TUI.
- **🎨 Colorful UI:** Vibrant, rainbow-styled interface with prominent active state indicators.
- **📊 VRAM Estimation:** Real-time estimation of VRAM usage for loaded models.
- **📝 Markdown Support:** Rich rendering of assistant responses including code blocks and formatting.
- **🔄 Streaming:** Real-time token streaming for immediate feedback.
- **⌨️ Shortcuts:** Context-aware keybinding help bar at the bottom of the screen.
- **🔗 Smart Context:** Automatically switches models when resuming old chat sessions.
- **🧹 Auto-Cleanup:** Deleting a model automatically removes its associated chat history.

## Installation

### Prerequisites

- [Ollama](https://ollama.com/) running locally (default: `http://localhost:11434`).
- Rust toolchain (cargo).

### Build from Source

```bash
git clone https://github.com/yourusername/vicuna.git
cd vicuna
cargo run --release
```

## Usage

- **Navigation:**
    - `Tab`: Switch between **Models** and **Chat** tabs.
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

GPL v3
