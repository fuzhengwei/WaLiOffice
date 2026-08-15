use crate::db::DbPool;
use crate::llm::LlmClient;
use once_cell::sync::Lazy;
use std::sync::Arc;

static DB_POOL: Lazy<DbPool> = Lazy::new(|| {
    let cfg = crate::config::config();
    crate::db::init_pool(&cfg.data_dir).expect("db init failed")
});

static LLM_CLIENT: Lazy<Arc<LlmClient>> = Lazy::new(|| Arc::new(LlmClient::new()));

pub fn db_pool() -> DbPool {
    DB_POOL.clone()
}

pub fn llm_client() -> Arc<LlmClient> {
    LLM_CLIENT.clone()
}
