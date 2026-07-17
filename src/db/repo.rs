use crate::api::types::{Model, ModelName, SessionId};
use crate::utils::vram;
use anyhow::Result;
use libsql::{Connection, named_params};
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

    let family = details.map(|d| d.family.clone());
    let param_size = details.map(|d| d.parameter_size.clone());
    let quant = details.map(|d| d.quantization_level.clone());

    let vram_usage = if let (Some(p), Some(q)) = (&param_size, &quant) {
        let p_val = vram::parse_model_params(p);
        let q_val = vram::parse_quantization(q);
        vram::estimate_vram_usage(p_val, q_val) as i64
    } else {
        0
    };

    conn.execute(
        "INSERT INTO models (name, family, size, quantization, params, modified_at, vram_usage)
         VALUES (:name, :family, :size, :quant, :params, :mod_at, :vram)
         ON CONFLICT(name) DO UPDATE SET
            family=excluded.family,
            size=excluded.size,
            quantization=excluded.quantization,
            params=excluded.params,
            modified_at=excluded.modified_at,
            vram_usage=excluded.vram_usage
        ",
        libsql::named_params! {
            ":name": model.name.clone(),
            ":family": family,
            ":size": model.size as i64,
            ":quant": quant,
            ":params": param_size,
            ":mod_at": model.modified_at.clone(),
            ":vram": vram_usage
        },
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
        "INSERT INTO sessions (id, title, model, created_at) VALUES (:id, :title, :model, datetime('now'))",
        named_params! {
            ":id": id.0.clone(),
            ":title": title,
            ":model": model.0.clone()
        },
    )
    .await?;
    Ok(())
}

pub async fn rename_session(conn: &Connection, id: &SessionId, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET title = :title WHERE id = :id",
        named_params! {
            ":id": id.0.clone(),
            ":title": title,
        },
    )
    .await?;
    Ok(())
}

pub async fn get_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let mut rows = conn
        .query(
            "SELECT id, title, model, created_at FROM sessions ORDER BY created_at DESC",
            (),
        )
        .await?;
    let mut sessions = Vec::new();
    while let Some(row) = rows.next().await? {
        sessions.push(Session {
            id: SessionId(row.get(0)?),
            title: row.get(1)?,
            model: ModelName(
                row.get::<Option<String>>(2)?
                    .unwrap_or_else(|| "unknown".into()),
            ),
            created_at: row.get(3)?,
        });
    }
    Ok(sessions)
}

pub async fn delete_model_cascade(conn: &Connection, name: &ModelName) -> Result<()> {
    conn.execute(
        "DELETE FROM sessions WHERE model = :name",
        named_params! { ":name": name.0.clone() },
    )
    .await?;
    conn.execute(
        "DELETE FROM models WHERE name = :name",
        named_params! { ":name": name.0.clone() },
    )
    .await?;
    Ok(())
}

pub async fn delete_session(conn: &Connection, id: &SessionId) -> Result<()> {
    conn.execute(
        "DELETE FROM sessions WHERE id = :id",
        named_params! { ":id": id.0.clone() },
    )
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
        "INSERT INTO messages (session_id, role, content, created_at) VALUES (:sid, :role, :content, datetime('now'))",
        named_params! {
            ":sid": session_id.0.clone(),
            ":role": role,
            ":content": content
        }
    ).await?;
    Ok(())
}

/// Remove the most recent assistant message for a session (regen / cancel cleanup).
pub async fn delete_last_assistant(conn: &Connection, session_id: &SessionId) -> Result<()> {
    conn.execute(
        "DELETE FROM messages WHERE id = (
            SELECT id FROM messages
            WHERE session_id = :sid AND role = 'assistant'
            ORDER BY id DESC
            LIMIT 1
        )",
        named_params! { ":sid": session_id.0.clone() },
    )
    .await?;
    Ok(())
}

pub async fn get_messages(conn: &Connection, session_id: &SessionId) -> Result<Vec<Message>> {
    let mut rows = conn
        .query(
            "SELECT role, content FROM messages WHERE session_id = :sid ORDER BY id ASC",
            named_params! { ":sid": session_id.0.clone() },
        )
        .await?;

    let mut messages = Vec::new();
    while let Some(row) = rows.next().await? {
        messages.push(Message {
            role: row.get(0)?,
            content: row.get(1)?,
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

        delete_last_assistant(&conn, &sid).await.unwrap();
        let msgs = get_messages(&conn, &sid).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");

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
