use anyhow::Result;
use libsql::Connection;
use std::path::Path;

pub mod repo;
pub mod schema;

pub async fn init(db_path: &Path) -> Result<Connection> {
    let db = libsql::Builder::new_local(db_path).build().await?;
    let conn = db.connect()?;

    schema::migrate(&conn).await?;

    Ok(conn)
}
