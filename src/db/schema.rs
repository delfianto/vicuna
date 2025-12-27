use anyhow::Result;
use libsql::Connection;

pub async fn migrate(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS models (
            name TEXT PRIMARY KEY,
            family TEXT,
            size TEXT,
            quantization TEXT,
            params TEXT,
            modified_at TEXT,
            vram_usage INTEGER
        )",
        (),
    )
    .await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT,
            model TEXT,
            created_at TEXT
        )",
        (),
    )
    .await?;

    // Migration: Add model column if it doesn't exist
    // SQLite doesn't support IF NOT EXISTS for ADD COLUMN, so we catch the error or check pragma.
    // However, a simple way is to just try it and ignore error "duplicate column name".
    // But logically, since we are in a tool loop, let's keep it simple.
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN model TEXT", ()).await;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT,
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
        (),
    )
    .await?;

    Ok(())
}
