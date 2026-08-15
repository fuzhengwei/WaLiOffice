pub mod notification_repo;
pub mod project_repo;
pub mod session_repo;
pub mod settings_repo;
pub mod task_repo;
pub mod user_repo;

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::fs;
use tracing::info;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConn = r2d2::PooledConnection<SqliteConnectionManager>;

pub fn init_pool(data_dir: &str) -> Result<DbPool> {
    fs::create_dir_all(data_dir)?;
    let db_path = format!("{data_dir}/walioffice.db");
    let manager = SqliteConnectionManager::file(&db_path);
    let pool = Pool::builder().max_size(8).build(manager)?;

    run_migrations(&pool)?;
    info!("📦 SQLite 数据库已初始化: {db_path}");
    Ok(pool)
}

fn run_migrations(pool: &DbPool) -> Result<()> {
    let sql = include_str!("../../../migrations/001_init.sql");
    let conn = pool.get()?;
    conn.execute_batch(sql)?;

    // seed admin user
    let admin_username = crate::config::config().admin_username.clone();
    let admin_password = crate::config::config().admin_password.clone();
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE username = ?1",
            params![&admin_username],
            |row| row.get(0),
        )
        .ok();

    if existing.is_none() {
        let id = "admin-001";
        let now = chrono::Utc::now().to_rfc3339();
        let hash = bcrypt::hash(&admin_password, 10)?;
        conn.execute(
            "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                &admin_username,
                "admin@walioffice.local",
                &hash,
                "admin",
                &now,
                &now
            ],
        )?;
        info!("✅ 已初始化管理员账号: {admin_username} / {admin_password}");
    }
    Ok(())
}
