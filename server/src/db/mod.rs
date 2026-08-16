pub mod file_repo;
pub mod notification_repo;
pub mod project_repo;
pub mod session_repo;
pub mod settings_repo;
pub mod user_repo;

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
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

fn ensure_session_order_col(conn: &DbConn) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(sessions)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_order_col = false;
    for column in columns {
        if column? == "order_col" {
            has_order_col = true;
            break;
        }
    }
    if !has_order_col {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN order_col INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

fn run_migrations(pool: &DbPool) -> Result<()> {
    let sql = include_str!("../../../migrations/001_init.sql");
    let conn = pool.get()?;
    conn.execute_batch(sql)?;
    ensure_session_order_col(&conn)?;
    Ok(())
}
