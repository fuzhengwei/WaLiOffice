use crate::db::DbPool;
use crate::llm::LlmClient;
use once_cell::sync::Lazy;
use std::sync::OnceLock;
use std::sync::Arc;

static DB_POOL: OnceLock<DbPool> = OnceLock::new();

static LLM_CLIENT: Lazy<Arc<LlmClient>> = Lazy::new(|| Arc::new(LlmClient::new()));

pub async fn init_db_pool() -> DbPool {
    let cfg = crate::config::config();
    crate::db::init_pool(&cfg.database_url, &cfg.data_dir, cfg.db_max_connections)
        .await
        .expect("db init failed")
}

pub fn db_pool() -> DbPool {
    DB_POOL
        .get()
        .expect("db pool not initialized — call init_db_pool() first")
        .clone()
}

pub fn set_db_pool(pool: DbPool) {
    DB_POOL.set(pool).expect("db pool already set");
}

pub fn llm_client() -> Arc<LlmClient> {
    LLM_CLIENT.clone()
}
