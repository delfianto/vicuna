# Vicuna: A High-Performance Rust TUI for Ollama

**Vicuna** is a feature-rich, terminal-based client for [Ollama](https://ollama.com/), built in **Rust**. It aims to provide a robust "OpenWebUI-like" experience directly in the terminal, focusing on speed, type safety, and efficient resource management.

---

## 1. Project Philosophy & Goals

- **Speed & Efficiency:** Leveraging Rust's zero-cost abstractions to ensure the UI remains responsive (60 FPS) even while streaming heavy tokens or managing massive chat histories.
- **Data Integrity:** Utilizing **SQLite (via Turso/LibSQL)** for structured, queryable storage of sessions and message history, rather than flat JSON files.
- **Modern Architecture:** Employing an **Async MPSC Actor** pattern to strictly separate UI rendering from blocking I/O operations.
- **Feature Parity:** Matching and exceeding features found in `gollama` and `parllama`, including VRAM estimation, model management, and markdown rendering.

---

## 2. Technology Stack

We target **Rust 2021/2024 Edition** standards, utilizing the latest stable crates.

| Category          | Crate              | Description                                                                |
| :---------------- | :----------------- | :------------------------------------------------------------------------- |
| **Language**      | `rustc`            | Edition 2021 (using modern idioms)                                         |
| **TUI Engine**    | `ratatui`          | The industry-standard fork of `tui-rs` for rendering.                      |
| **Terminal**      | `crossterm`        | Handling raw mode, events, and cross-platform terminal control.            |
| **Async Runtime** | `tokio`            | Full features (`rt-multi-thread`, `macros`) for async I/O.                 |
| **HTTP Client**   | `reqwest`          | Using `stream` features for handling Server-Sent Events (SSE) from Ollama. |
| **Database**      | `libsql`           | The Rust SDK for Turso/SQLite. operating in embedded file mode.            |
| **Logging**       | `tracing`          | Structured logging with `tracing-appender` for file rotation.              |
| **Markdown**      | `ratatui-markdown` | Parsing and rendering Markdown syntax in TUI widgets.                      |
| **Input**         | `tui-textarea`     | A robust text editor widget for the chat input field.                      |
| **Config**        | `directories`      | XDG-compliant path resolution (`~/.config/vicuna/`).                       |

---

## 3. Architecture Design

### 3.1 The Actor Pattern

To prevent the UI from freezing during network requests, **Vicuna** uses a message-passing architecture:

1.  **Main Thread (UI):** Runs the `ratatui` draw loop. It captures keyboard input and sends **Actions** (Enums) to the backend. It receives **Events** (Enums) to update state.
2.  **Backend Actor (Tokio Task):** Listens on an `mpsc::channel`. Handles `reqwest` calls to Ollama and database writes.
3.  **Streaming:** When generating chat responses, the Backend Actor pushes individual `Token(String)` events to the Main Thread via a dedicated channel to ensure immediate visual feedback.

### 3.2 Database Schema

Data is stored in `~/.config/vicuna/vicuna.db` (SQLite).

**Tables:**

- `models`: Caches metadata (Name, Quantization, VRAM usage) to reduce API polling.
- `sessions`: Stores Chat IDs, Titles, and Timestamps.
- `messages`: Stores role (user/assistant), content, and foreign key to `sessions`.

---

## 4. Directory Structure

```text
src/
├── main.rs              # Entry point, Panic Hooks, TUI initialization
├── app.rs               # Main Application State & Event Loop
├── config.rs            # Configuration loading & XDG path resolution
├── logging.rs           # Tracing setup & log rotation
├── db/
│   ├── mod.rs           # Libsql connection builder
│   ├── schema.rs        # "CREATE TABLE" SQL definitions
│   └── repo.rs          # Database CRUD operations (Repository pattern)
├── api/
│   ├── client.rs        # Reqwest logic for Ollama API
│   ├── types.rs         # Serde structs (OllamaResponse, GenerateRequest)
│   └── modelfile.rs     # Parser for GGUF metadata (Quantization/Params)
├── tui/
│   ├── mod.rs           # Terminal setup/teardown boilerplate
│   ├── events.rs        # KeyMapping and Event definitions
│   ├── styles.rs        # Centralized color themes (Vicuna Theme)
│   ├── components/
│   │   ├── markdown.rs  # Markdown widget wrapper
│   │   ├── popup.rs     # Generic popup dialog (Inputs/Confirmations)
│   │   └── toast.rs     # Notification system
│   └── tabs/
│       ├── models.rs    # Model Management UI logic
│       └── chat.rs      # Chat UI logic (Split panes)
└── utils/
    └── vram.rs          # VRAM estimation math logic
```

---

## 5. Implementation Plan

### Phase 1: Foundation & Infrastructure

- [ ] **Init:** `cargo new vicuna` and add dependencies.
- [ ] **Logging:** Implement `logging.rs` using `tracing_appender` to write daily logs to `~/.config/vicuna/logs/vicuna.log`. Ensure **no** logs print to stdout.
- [ ] **Config:** Implement `config.rs` using `directories` crate to establish `config_dir` and `data_dir`.
- [ ] **Database:** Implement `db/mod.rs` to initialize `libsql`. Add migration logic in `db/schema.rs` to create tables if they don't exist.

### Phase 2: The Core Backend

- [ ] **Ollama Client:** Create `api/client.rs`. Implement:
- `check_health()`: `GET /`
- `list_models()`: `GET /api/tags`
- `show_model()`: `POST /api/show`

- [ ] **Model Parser:** Implement logic in `api/modelfile.rs` to parse the `Modelfile` string and extract parameter counts (e.g., "7B") and quantization (e.g., "Q4_0").
- [ ] **Repository:** Implement `db/repo.rs` with methods:
- `upsert_model()`
- `create_session()`
- `get_sessions()`

### Phase 3: UI - Model Management

- [ ] **TUI Boilerplate:** Set up the generic `Terminal<CrosstermBackend>` loop in `main.rs`.
- [ ] **Model Table:** In `tui/tabs/models.rs`, create a `Table` widget.
- Columns: Name, Family, Size, Quant, Params, Modified.

- [ ] **Sorting:** Implement `PartialOrd` for the Model struct to allow sorting by Name/Size/Date.
- [ ] **VRAM Calc:** Implement `utils/vram.rs`.
- Formula: `(Params * Quant_Bits / 8) + Context_Window_Overhead`.
- Display "Est. VRAM" in a side panel.

- [ ] **Actions:** Wire up 'D' (Delete) and 'P' (Pull) keys to async functions in the backend.

### Phase 4: UI - Chat Interface

- [ ] **Layout:** Create `tui/tabs/chat.rs`. Use `Layout::horizontal` to split Session List (20%) and Chat Area (80%).
- [ ] **Input:** Integrate `tui-textarea` at the bottom of the Chat Area.
- [ ] **Streaming Pipeline:**
- Implement `api::client::generate_stream`.
- Create an `Action::TokenReceived(String)` event.
- On `TokenReceived`, append text to the current message in `AppState` and force a UI redraw.

- [ ] **Markdown:** Implement `tui/components/markdown.rs` using `ratatui-markdown` to render the `Assistant` messages.

### Phase 5: Refinement & Polish

- [ ] **Toast System:** Create a global overlay for errors (e.g., "Connection Refused").
- [ ] **Keybindings:** Standardize navigation (`j`/`k` for lists, `Tab` to switch views, `?` for help).
- [ ] **Optimization:** Ensure the `libsql` writes for chat history happen _after_ the stream finishes, or batch them, to avoid locking the DB during generation.
- [ ] **CI/CD:** Add a generic GitHub Action for `cargo test` and `clippy`.

---

## 6. Development Guidelines

- **Linting:** `cargo clippy` must pass with no warnings.
- **Error Handling:** Use `anyhow::Result` for the backend, but strictly handle UI errors gracefully (show a Toast, don't crash).
- **Commits:** Follow "Conventional Commits" (e.g., `feat: add database schema`, `fix: text wrapping in chat`).
- **Testing:**
- Unit tests for `utils/vram.rs` and `api/modelfile.rs` are mandatory.
- Mocking the `OllamaClient` trait is required for testing UI logic without a running server.
