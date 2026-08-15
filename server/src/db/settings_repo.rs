use rusqlite::params;

use crate::error::AppResult;
use crate::models::AppSettings;

use super::DbPool;

pub fn find_by_user(pool: &DbPool, user_id: &str) -> AppResult<Option<AppSettings>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT payload FROM user_settings WHERE user_id = ?1",
        params![user_id],
        |row| row.get::<_, String>(0),
    );

    match row {
        Ok(payload) => Ok(Some(serde_json::from_str::<AppSettings>(&payload)?)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_for_user(
    pool: &DbPool,
    user_id: &str,
    settings: &AppSettings,
) -> AppResult<AppSettings> {
    let mut conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::to_string(settings)?;

    let tx = conn.transaction()?;
    let updated = tx.execute(
        "UPDATE user_settings
         SET payload = ?1, updated_at = ?2
         WHERE user_id = ?3",
        params![&payload, &now, user_id],
    )?;

    if updated == 0 {
        tx.execute(
            "INSERT INTO user_settings (id, user_id, payload, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                user_id,
                &payload,
                &now,
                &now
            ],
        )?;
    }

    tx.commit()?;

    Ok(settings.clone())
}
