use crate::db::DbPool;
use crate::error::AppResult;
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FileRow {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub folder_id: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderRow {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileStats {
    pub by_type: HashMap<String, i64>,
    pub total_size: i64,
    pub total_files: i64,
}

pub async fn list_files(
    pool: &DbPool,
    owner_id: &str,
    folder_id: Option<&str>,
) -> AppResult<Vec<FileRow>> {
    let rows = if let Some(folder_id) = folder_id {
        sqlx::query(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files WHERE owner_id = ? AND folder_id = ? ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .bind(folder_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files WHERE owner_id = ? AND folder_id IS NULL ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .fetch_all(pool)
        .await?
    };

    let mut files = Vec::new();
    for row in rows {
        files.push(map_file(&row)?);
    }
    Ok(files)
}

pub async fn search_files(pool: &DbPool, owner_id: &str, query: Option<&str>) -> AppResult<Vec<FileRow>> {
    let pattern = format!("%{}%", query.unwrap_or("").trim());
    let rows = sqlx::query(
        "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
         FROM files
         WHERE owner_id = ? AND (? = '%%' OR name LIKE ? OR COALESCE(description, '') LIKE ?)
         ORDER BY updated_at DESC"
    )
    .bind(owner_id)
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let mut files = Vec::new();
    for row in rows {
        files.push(map_file(&row)?);
    }
    Ok(files)
}

pub async fn get_file(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FileRow>> {
    let row = sqlx::query(
        "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
         FROM files WHERE id = ? AND owner_id = ?"
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_file(&r)?)),
        None => Ok(None),
    }
}

async fn get_generated_artifact_file(
    pool: &DbPool,
    owner_id: &str,
    metadata: Option<&Value>,
) -> AppResult<Option<FileRow>> {
    let artifact_id = metadata
        .and_then(|value| value.get("artifact_id"))
        .and_then(|value| value.as_str());
    if artifact_id.is_none() {
        return Ok(None);
    }

    let cfg = crate::config::config();

    let row = if cfg.is_mysql() {
        sqlx::query(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files
             WHERE owner_id = ?
               AND JSON_EXTRACT(metadata, '$.source') = 'generated_artifact'
               AND JSON_EXTRACT(metadata, '$.artifact_id') = ?
             ORDER BY updated_at DESC
             LIMIT 1"
        )
        .bind(owner_id)
        .bind(artifact_id)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files
             WHERE owner_id = ?
               AND json_extract(metadata, '$.source') = 'generated_artifact'
               AND json_extract(metadata, '$.artifact_id') = ?
             ORDER BY updated_at DESC
             LIMIT 1"
        )
        .bind(owner_id)
        .bind(artifact_id)
        .fetch_optional(pool)
        .await?
    };

    match row {
        Some(r) => Ok(Some(map_file(&r)?)),
        None => Ok(None),
    }
}

pub async fn create_file(
    pool: &DbPool,
    owner_id: &str,
    name: &str,
    file_path: &str,
    file_type: &str,
    file_size: i64,
    folder_id: Option<&str>,
    description: Option<&str>,
    metadata: Option<Value>,
) -> AppResult<FileRow> {
    if let Some(existing) = get_generated_artifact_file(pool, owner_id, metadata.as_ref()).await? {
        return Ok(existing);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let metadata_text = metadata.map(|value| value.to_string());
    sqlx::query(
        "INSERT INTO files (id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(owner_id)
    .bind(name)
    .bind(file_path)
    .bind(file_type)
    .bind(file_size)
    .bind(folder_id)
    .bind(description)
    .bind(&metadata_text)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(FileRow {
        id,
        owner_id: owner_id.to_string(),
        name: name.to_string(),
        file_path: file_path.to_string(),
        file_type: file_type.to_string(),
        file_size,
        folder_id: folder_id.map(str::to_string),
        description: description.map(str::to_string),
        metadata: metadata_text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok()),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn delete_file(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FileRow>> {
    let file = match get_file(pool, owner_id, id).await? {
        Some(file) => file,
        None => return Ok(None),
    };
    sqlx::query("DELETE FROM files WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(owner_id)
        .execute(pool)
        .await?;
    Ok(Some(file))
}

pub async fn stats(pool: &DbPool, owner_id: &str) -> AppResult<FileStats> {
    let row = sqlx::query(
        "SELECT COUNT(*), COALESCE(SUM(file_size), 0) FROM files WHERE owner_id = ?"
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await?;

    let total_files: i64 = row.try_get(0)?;
    let total_size: i64 = row.try_get(1)?;

    let mut by_type = HashMap::new();
    let rows = sqlx::query(
        "SELECT file_type, COUNT(*) FROM files WHERE owner_id = ? GROUP BY file_type"
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let file_type: String = row.try_get(0)?;
        let count: i64 = row.try_get(1)?;
        by_type.insert(file_type, count);
    }

    Ok(FileStats {
        by_type,
        total_size,
        total_files,
    })
}

pub async fn list_folders(
    pool: &DbPool,
    owner_id: &str,
    parent_id: Option<&str>,
) -> AppResult<Vec<FolderRow>> {
    let rows = if let Some(parent_id) = parent_id {
        sqlx::query(
            "SELECT id, owner_id, name, parent_id, created_at, updated_at
             FROM folders WHERE owner_id = ? AND parent_id = ? ORDER BY name ASC"
        )
        .bind(owner_id)
        .bind(parent_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, owner_id, name, parent_id, created_at, updated_at
             FROM folders WHERE owner_id = ? AND parent_id IS NULL ORDER BY name ASC"
        )
        .bind(owner_id)
        .fetch_all(pool)
        .await?
    };

    let mut folders = Vec::new();
    for row in rows {
        folders.push(map_folder(&row)?);
    }
    Ok(folders)
}

pub async fn get_folder(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FolderRow>> {
    let row = sqlx::query(
        "SELECT id, owner_id, name, parent_id, created_at, updated_at
         FROM folders WHERE id = ? AND owner_id = ?"
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_folder(&r)?)),
        None => Ok(None),
    }
}

pub async fn create_folder(
    pool: &DbPool,
    owner_id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> AppResult<FolderRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO folders (id, owner_id, name, parent_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(owner_id)
    .bind(name)
    .bind(parent_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(FolderRow {
        id,
        owner_id: owner_id.to_string(),
        name: name.to_string(),
        parent_id: parent_id.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn delete_folder_tree(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Vec<FileRow>> {
    if get_folder(pool, owner_id, id).await?.is_none() {
        return Ok(Vec::new());
    }

    let mut folder_ids = vec![id.to_string()];
    let mut index = 0;
    while index < folder_ids.len() {
        let parent_id = folder_ids[index].clone();
        let rows = sqlx::query(
            "SELECT id FROM folders WHERE owner_id = ? AND parent_id = ?"
        )
        .bind(owner_id)
        .bind(&parent_id)
        .fetch_all(pool)
        .await?;

        for row in rows {
            let child_id: String = row.try_get(0)?;
            folder_ids.push(child_id);
        }
        index += 1;
    }

    let mut removed_files = Vec::new();
    for folder_id in &folder_ids {
        removed_files.extend(list_files(pool, owner_id, Some(folder_id)).await?);
    }

    for file in &removed_files {
        sqlx::query("DELETE FROM files WHERE id = ? AND owner_id = ?")
            .bind(&file.id)
            .bind(owner_id)
            .execute(pool)
            .await?;
    }
    for folder_id in folder_ids.iter().rev() {
        sqlx::query("DELETE FROM folders WHERE id = ? AND owner_id = ?")
            .bind(folder_id)
            .bind(owner_id)
            .execute(pool)
            .await?;
    }

    Ok(removed_files)
}

fn map_file(row: &sqlx::any::AnyRow) -> AppResult<FileRow> {
    let metadata_text: Option<String> = row.try_get(8)?;
    let metadata = metadata_text
        .as_deref()
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    Ok(FileRow {
        id: row.try_get(0)?,
        owner_id: row.try_get(1)?,
        name: row.try_get(2)?,
        file_path: row.try_get(3)?,
        file_type: row.try_get(4)?,
        file_size: row.try_get(5)?,
        folder_id: row.try_get(6)?,
        description: row.try_get(7)?,
        metadata,
        created_at: row.try_get(9)?,
        updated_at: row.try_get(10)?,
    })
}

fn map_folder(row: &sqlx::any::AnyRow) -> AppResult<FolderRow> {
    Ok(FolderRow {
        id: row.try_get(0)?,
        owner_id: row.try_get(1)?,
        name: row.try_get(2)?,
        parent_id: row.try_get(3)?,
        created_at: row.try_get(4)?,
        updated_at: row.try_get(5)?,
    })
}
