use crate::api::types::{Model, ModelName, SessionId};
use crate::utils::vram;
use anyhow::Result;
use libsql::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub title: String,
    pub model: ModelName,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct Repository {
    pub conn: Connection,
}

impl Repository {
    pub async fn new(db_path: &std::path::Path) -> Result<Self> {
        let db = libsql::Builder::new_local(db_path).build().await?;
        let conn = db.connect()?;
        crate::db::schema::migrate(&conn).await?;
        Ok(Self { conn })
    }
}

pub async fn upsert_model(conn: &Connection, model: &Model) -> Result<()> {
    let details = model.details.as_ref();
    let family = details.map(|d| d.family.as_str());
    let param_size = details.map(|d| d.parameter_size.as_str());
    let quant = details.map(|d| d.quantization_level.as_str());

    let p_val = vram::parse_model_params(param_size.unwrap_or_default());
    let q_val = vram::parse_quantization(quant.unwrap_or_default());
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

pub async fn create_session(
    conn: &Connection,
    id: &SessionId,
    title: &str,
    model: &ModelName,
) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, title, model, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
        params![id.0.clone(), title, model.0.clone()],
    )
    .await?;
    Ok(())
}

pub async fn get_sessions(conn: &Connection) -> Result<Vec<Session>> {
    const COL_ID: i32 = 0;
    const COL_TITLE: i32 = 1;
    const COL_MODEL: i32 = 2;
    const COL_CREATED_AT: i32 = 3;

    let mut rows = conn
        .query(
            "SELECT id, title, model, created_at FROM sessions ORDER BY created_at DESC",
            (),
        )
        .await?;
    let mut sessions = Vec::new();
    while let Some(row) = rows.next().await? {
        sessions.push(Session {
            id: SessionId(row.get(COL_ID)?),
            title: row.get(COL_TITLE)?,
            model: ModelName(
                row.get::<Option<String>>(COL_MODEL)?
                    .unwrap_or_else(|| "unknown".into()),
            ),
            created_at: row.get(COL_CREATED_AT)?,
        });
    }
    Ok(sessions)
}

pub async fn delete_model_cascade(conn: &Connection, name: &ModelName) -> Result<()> {
    conn.execute(
        "DELETE FROM sessions WHERE model = ?1",
        params![name.0.clone()],
    )
    .await?;
    conn.execute(
        "DELETE FROM models WHERE name = ?1",
        params![name.0.clone()],
    )
    .await?;
    Ok(())
}

pub async fn delete_session(conn: &Connection, id: &SessionId) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id.0.clone()])
        .await?;
    Ok(())
}

pub async fn add_message(
    conn: &Connection,
    session_id: &SessionId,
    role: &str,
    content: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, datetime('now'))",
        params![session_id.0.clone(), role, content]
    ).await?;
    Ok(())
}

pub async fn get_messages(conn: &Connection, session_id: &SessionId) -> Result<Vec<Message>> {
    const COL_ROLE: i32 = 0;
    const COL_CONTENT: i32 = 1;

    let mut rows = conn
        .query(
            "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY id ASC",
            params![session_id.0.clone()],
        )
        .await?;

    let mut messages = Vec::new();
    while let Some(row) = rows.next().await? {
        messages.push(Message {
            role: row.get(COL_ROLE)?,
            content: row.get(COL_CONTENT)?,
        });
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
        let sid = SessionId("s1".to_string());
        let mname = ModelName("llama3".to_string());

        create_session(&conn, &sid, "Test Session", &mname)
            .await
            .unwrap();

        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id.0, "s1");
        assert_eq!(sessions[0].title, "Test Session");
        assert_eq!(sessions[0].model.0, "llama3");

        add_message(&conn, &sid, "user", "hello").await.unwrap();
        add_message(&conn, &sid, "assistant", "hi").await.unwrap();

        let msgs = get_messages(&conn, &sid).await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");

        delete_session(&conn, &sid).await.unwrap();
        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_model_cascade_delete() {
        let conn = setup_db().await;
        let sid1 = SessionId("s1".to_string());
        let sid2 = SessionId("s2".to_string());
        let mname_a = ModelName("model-a".to_string());
        let mname_b = ModelName("model-b".to_string());

        create_session(&conn, &sid1, "Session 1", &mname_a)
            .await
            .unwrap();
        create_session(&conn, &sid2, "Session 2", &mname_b)
            .await
            .unwrap();

        delete_model_cascade(&conn, &mname_a).await.unwrap();

        let sessions = get_sessions(&conn).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].model.0, "model-b");
    }
}
