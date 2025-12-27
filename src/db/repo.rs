use crate::api::types::Model;
use crate::utils::vram;
use anyhow::Result;
use libsql::{params, Connection};

pub async fn upsert_model(conn: &Connection, model: &Model) -> Result<()> {
    let family = model
        .details
        .as_ref()
        .map(|d| d.family.clone())
        .unwrap_or_default();
    let param_size = model
        .details
        .as_ref()
        .map(|d| d.parameter_size.clone())
        .unwrap_or_default();
    let quant = model
        .details
        .as_ref()
        .map(|d| d.quantization_level.clone())
        .unwrap_or_default();

    let p_val = vram::parse_model_params(&param_size);
    let q_val = vram::parse_quantization(&quant);
    let vram_usage = vram::estimate_vram_usage(p_val, q_val) as i64;

    conn.execute(
        "INSERT INTO models (name, family, size, quantization, params, modified_at, vram_usage)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(name) DO UPDATE SET
            family=excluded.family,
            size=excluded.size,
            quantization=excluded.quantization,
            params=excluded.params,
            modified_at=excluded.modified_at,
            vram_usage=excluded.vram_usage
        ",
        params![
            model.name.clone(),
            family,
            model.size.to_string(),
            quant,
            param_size,
            model.modified_at.clone(),
            vram_usage
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_session(conn: &Connection, id: &str, title: &str, model: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, title, model, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
        params![id, title, model],
    )
    .await?;
    Ok(())
}

pub async fn get_sessions(conn: &Connection) -> Result<Vec<(String, String, String, String)>> {
    let mut rows = conn
        .query(
            "SELECT id, title, model, created_at FROM sessions ORDER BY created_at DESC",
            (),
        )
        .await?;
    let mut sessions = Vec::new();
    while let Some(row) = rows.next().await? {
        let model: String = row
            .get::<Option<String>>(2)?
            .unwrap_or("unknown".to_string());
        sessions.push((row.get(0)?, row.get(1)?, model, row.get(3)?));
    }
    Ok(sessions)
}

pub async fn delete_model_cascade(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE model = ?1", params![name])
        .await?;
    conn.execute("DELETE FROM models WHERE name = ?1", params![name])
        .await?;
    Ok(())
}

pub async fn delete_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
        .await?;
    Ok(())
}

pub async fn add_message(
    conn: &Connection,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
        params![session_id, role, content]
    ).await?;
    Ok(())
}

pub async fn get_messages(conn: &Connection, session_id: &str) -> Result<Vec<(String, String)>> {
    let mut rows = conn
        .query(
            "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY id ASC",
            params![session_id],
        )
        .await?;

    let mut messages = Vec::new();
    while let Some(row) = rows.next().await? {
        messages.push((row.get(0)?, row.get(1)?));
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    async fn setup_db() -> Connection {
        let db = libsql::Builder::new_local(":memory:")
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        schema::migrate(&conn).await.unwrap();
        conn
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let conn = setup_db().await;

        create_session(&conn, "s1", "Test Session", "llama3")
            .await
            .unwrap();

        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].0, "s1");
        assert_eq!(sessions[0].1, "Test Session");
        assert_eq!(sessions[0].2, "llama3");

        add_message(&conn, "s1", "user", "hello").await.unwrap();
        add_message(&conn, "s1", "assistant", "hi").await.unwrap();

        let msgs = get_messages(&conn, "s1").await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].0, "user");
        assert_eq!(msgs[1].0, "assistant");

        delete_session(&conn, "s1").await.unwrap();
        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_model_cascade_delete() {
        let conn = setup_db().await;

        create_session(&conn, "s1", "Session 1", "model-a")
            .await
            .unwrap();
        create_session(&conn, "s2", "Session 2", "model-b")
            .await
            .unwrap();

        delete_model_cascade(&conn, "model-a").await.unwrap();

        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].2, "model-b");
    }
}