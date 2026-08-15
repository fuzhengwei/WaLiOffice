use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub app_name: String,
    pub host: String,
    pub port: u16,

    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,

    pub llm_base_url: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub llm_provider: String,
    pub llm_tool_timeout_ms: u64,
    pub llm_chat_timeout_ms: u64,

    pub data_dir: String,
    pub projects_dir: String,
    pub sessions_dir: String,
    pub render_output_dir: String,

    pub cors_origins: Vec<String>,

    pub admin_username: String,
    pub admin_password: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let jwt_secret = env_or_required("AIPPT_JWT_SECRET");
        let llm_api_key = env_or_required("AIPPT_LLM_API_KEY");

        Ok(Self {
            app_name: env_or("AIPPT_APP_NAME", "WaLiOffice"),
            host: env_or("AIPPT_HOST", "0.0.0.0"),
            port: env_or("AIPPT_PORT", "8000").parse().unwrap_or(8000),

            jwt_secret,
            jwt_expiry_hours: env_or("AIPPT_JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),

            llm_base_url: env_or("AIPPT_LLM_BASE_URL", "http://127.0.0.1:8777/v1"),
            llm_api_key,
            llm_model: env_or("AIPPT_LLM_MODEL", "gpt-5.5"),
            llm_provider: env_or("AIPPT_LLM_PROVIDER", "glm-gateway"),
            llm_tool_timeout_ms: env_or("AIPPT_LLM_TOOL_TIMEOUT_MS", "600000")
                .parse()
                .unwrap_or(600_000),
            llm_chat_timeout_ms: env_or("AIPPT_LLM_CHAT_TIMEOUT_MS", "600000")
                .parse()
                .unwrap_or(600_000),

            data_dir: env_or("AIPPT_DATA_DIR", "data"),
            projects_dir: env_or("AIPPT_PROJECTS_DIR", "data/projects"),
            sessions_dir: env_or("AIPPT_SESSIONS_DIR", "data/sessions"),
            render_output_dir: env_or("AIPPT_RENDER_OUTPUT_DIR", "outputs"),

            cors_origins: env_or("AIPPT_CORS_ORIGINS", "http://localhost:5173")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),

            admin_username: env_or("AIPPT_ADMIN_USERNAME", "admin"),
            admin_password: env_or("AIPPT_ADMIN_PASSWORD", "admin123"),
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in &[
            &self.data_dir,
            &self.projects_dir,
            &self.sessions_dir,
            &self.render_output_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("缺少必要环境变量：{key}。请复制 .env.example 为 .env 并填写配置。");
        std::process::exit(1);
    })
}

use std::sync::OnceLock;
static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get_or_init(|| Config::from_env().expect("config init failed"))
}
