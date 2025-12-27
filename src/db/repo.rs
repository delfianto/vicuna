use crate::api::types::Model;
use anyhow::Result;
use libsql::{Connection, params};

#[allow(dead_code)]
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

    // vram_usage is calculated elsewhere, for now 0.
    let vram_usage: i64 = 0;

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

#[allow(dead_code)]
pub async fn create_session(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, title, created_at) VALUES (?1, ?2, datetime('now'))",
        params![id, title],
    )
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn get_sessions(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut rows = conn
        .query(
            "SELECT id, title, created_at FROM sessions ORDER BY created_at DESC",
            (),
        )
        .await?;
    let mut sessions = Vec::new();
    while let Some(row) = rows.next().await? {
        sessions.push((row.get(0)?, row.get(1)?, row.get(2)?));
    }
    Ok(sessions)
}
