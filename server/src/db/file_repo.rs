use crate::db::DbPool;
use crate::error::AppResult;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
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

fn map_file(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRow> {
    let metadata_text: Option<String> = row.get(8)?;
    let metadata = metadata_text
        .as_deref()
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    Ok(FileRow {
        id: row.get(0)?,
        owner_id: row.get(1)?,
        name: row.get(2)?,
        file_path: row.get(3)?,
        file_type: row.get(4)?,
        file_size: row.get(5)?,
        folder_id: row.get(6)?,
        description: row.get(7)?,
        metadata,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_folder(row: &rusqlite::Row<'_>) -> rusqlite::Result<FolderRow> {
    Ok(FolderRow {
        id: row.get(0)?,
        owner_id: row.get(1)?,
        name: row.get(2)?,
        parent_id: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub fn list_files(
    pool: &DbPool,
    owner_id: &str,
    folder_id: Option<&str>,
) -> AppResult<Vec<FileRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut files = Vec::new();
    if let Some(folder_id) = folder_id {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files WHERE owner_id = ?1 AND folder_id = ?2 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![owner_id, folder_id], map_file)?;
        for row in rows {
            files.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files WHERE owner_id = ?1 AND folder_id IS NULL ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![owner_id], map_file)?;
        for row in rows {
            files.push(row?);
        }
    }
    Ok(files)
}

pub fn search_files(pool: &DbPool, owner_id: &str, query: Option<&str>) -> AppResult<Vec<FileRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let pattern = format!("%{}%", query.unwrap_or("").trim());
    let mut stmt = conn.prepare(
        "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
         FROM files
         WHERE owner_id = ?1 AND (?2 = '%%' OR name LIKE ?2 OR COALESCE(description, '') LIKE ?2)
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map(params![owner_id, pattern], map_file)?;
    let mut files = Vec::new();
    for row in rows {
        files.push(row?);
    }
    Ok(files)
}

pub fn get_file(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FileRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let file = conn
        .query_row(
            "SELECT id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at
             FROM files WHERE id = ?1 AND owner_id = ?2",
            params![id, owner_id],
            map_file,
        )
        .optional()?;
    Ok(file)
}

pub fn create_file(
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
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let metadata_text = metadata.map(|value| value.to_string());
    conn.execute(
        "INSERT INTO files (id, owner_id, name, file_path, file_type, file_size, folder_id, description, metadata, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            &id,
            owner_id,
            name,
            file_path,
            file_type,
            file_size,
            folder_id,
            description,
            metadata_text,
            &now,
            &now
        ],
    )?;
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

pub fn delete_file(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FileRow>> {
    let file = match get_file(pool, owner_id, id)? {
        Some(file) => file,
        None => return Ok(None),
    };
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    conn.execute(
        "DELETE FROM files WHERE id = ?1 AND owner_id = ?2",
        params![id, owner_id],
    )?;
    Ok(Some(file))
}

pub fn stats(pool: &DbPool, owner_id: &str) -> AppResult<FileStats> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let (total_files, total_size): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(file_size), 0) FROM files WHERE owner_id = ?1",
        params![owner_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut by_type = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT file_type, COUNT(*) FROM files WHERE owner_id = ?1 GROUP BY file_type")?;
    let rows = stmt.query_map(params![owner_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (file_type, count) = row?;
        by_type.insert(file_type, count);
    }
    Ok(FileStats {
        by_type,
        total_size,
        total_files,
    })
}

pub fn list_folders(
    pool: &DbPool,
    owner_id: &str,
    parent_id: Option<&str>,
) -> AppResult<Vec<FolderRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut folders = Vec::new();
    if let Some(parent_id) = parent_id {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, name, parent_id, created_at, updated_at
             FROM folders WHERE owner_id = ?1 AND parent_id = ?2 ORDER BY name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map(params![owner_id, parent_id], map_folder)?;
        for row in rows {
            folders.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, name, parent_id, created_at, updated_at
             FROM folders WHERE owner_id = ?1 AND parent_id IS NULL ORDER BY name COLLATE NOCASE ASC",
        )?;
        let rows = stmt.query_map(params![owner_id], map_folder)?;
        for row in rows {
            folders.push(row?);
        }
    }
    Ok(folders)
}

pub fn get_folder(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Option<FolderRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let folder = conn
        .query_row(
            "SELECT id, owner_id, name, parent_id, created_at, updated_at
             FROM folders WHERE id = ?1 AND owner_id = ?2",
            params![id, owner_id],
            map_folder,
        )
        .optional()?;
    Ok(folder)
}

pub fn create_folder(
    pool: &DbPool,
    owner_id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> AppResult<FolderRow> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO folders (id, owner_id, name, parent_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![&id, owner_id, name, parent_id, &now, &now],
    )?;
    Ok(FolderRow {
        id,
        owner_id: owner_id.to_string(),
        name: name.to_string(),
        parent_id: parent_id.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn delete_folder_tree(pool: &DbPool, owner_id: &str, id: &str) -> AppResult<Vec<FileRow>> {
    if get_folder(pool, owner_id, id)?.is_none() {
        return Ok(Vec::new());
    }

    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut folder_ids = vec![id.to_string()];
    let mut index = 0;
    while index < folder_ids.len() {
        let parent_id = folder_ids[index].clone();
        let mut stmt =
            conn.prepare("SELECT id FROM folders WHERE owner_id = ?1 AND parent_id = ?2")?;
        let rows = stmt.query_map(params![owner_id, parent_id], |row| row.get::<_, String>(0))?;
        for row in rows {
            folder_ids.push(row?);
        }
        index += 1;
    }

    let mut removed_files = Vec::new();
    for folder_id in &folder_ids {
        removed_files.extend(list_files(pool, owner_id, Some(folder_id))?);
    }

    for file in &removed_files {
        conn.execute(
            "DELETE FROM files WHERE id = ?1 AND owner_id = ?2",
            params![&file.id, owner_id],
        )?;
    }
    for folder_id in folder_ids.iter().rev() {
        conn.execute(
            "DELETE FROM folders WHERE id = ?1 AND owner_id = ?2",
            params![folder_id, owner_id],
        )?;
    }

    Ok(removed_files)
}
