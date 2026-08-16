use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub app_name: String,
    pub host: String,
    pub port: u16,

    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub x_api_auth_login_url: String,

    pub llm_base_url: String,
    pub llm_api_key: String,
    pub llm_api_keys: Vec<String>,
    pub llm_model: String,
    pub llm_text_base_url: String,
    pub llm_text_api_key: String,
    pub llm_text_api_keys: Vec<String>,
    pub llm_text_model: String,
    pub llm_text_models: Vec<String>,
    pub llm_image_base_url: String,
    pub llm_image_api_key: String,
    pub llm_image_api_keys: Vec<String>,
    pub llm_image_model: String,
    pub llm_image_models: Vec<String>,
    pub llm_video_base_url: String,
    pub llm_video_api_key: String,
    pub llm_video_api_keys: Vec<String>,
    pub llm_video_model: String,
    pub llm_video_models: Vec<String>,
    pub llm_provider: String,
    pub llm_tool_timeout_ms: u64,
    pub llm_chat_timeout_ms: u64,
    pub web_search_provider: String,
    pub web_search_endpoint: String,
    pub web_search_timeout_ms: u64,
    pub baidu_mcp_api_key: String,
    pub baidu_mcp_sse_endpoint: String,

    pub data_dir: String,
    pub projects_dir: String,
    pub sessions_dir: String,
    pub render_output_dir: String,

    pub cors_origins: Vec<String>,

    // 数据库配置
    pub database_url: String,
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let jwt_secret = env_or_required("AIPPT_JWT_SECRET");

        let llm_text_base_url = env_or_required("LLM_TEXT_BASE_URL");
        let llm_text_api_key = env_or_required("LLM_TEXT_API_KEY");
        let llm_text_api_keys = split_api_keys(&llm_text_api_key);

        let llm_image_base_url = env_or_required("LLM_IMAGE_BASE_URL");
        let llm_image_api_key = env_or_required("LLM_IMAGE_API_KEY");
        let llm_image_api_keys = split_api_keys(&llm_image_api_key);

        let llm_video_base_url = env_or_required("LLM_VIDEO_BASE_URL");
        let llm_video_api_key = env_or_required("LLM_VIDEO_API_KEY");
        let llm_video_api_keys = split_api_keys(&llm_video_api_key);

        // 模型列表（必填，逗号分隔）；默认模型取列表首个或 *_MODELS_DEFAULT
        // 向后兼容：如果没配 LLM_TEXT_MODELS 但配了 LLM_TEXT_MODEL，则用后者作为单元素列表
        let llm_text_models = match env::var("LLM_TEXT_MODELS") {
            Ok(v) if !v.trim().is_empty() => split_env_list(&v),
            _ => vec![env_or_required("LLM_TEXT_MODEL")],
        };
        let llm_text_model = {
            let d = env_or("LLM_TEXT_MODELS_DEFAULT", "");
            if d.trim().is_empty() {
                llm_text_models.first().cloned().unwrap_or_default()
            } else {
                d
            }
        };

        let llm_image_models = match env::var("LLM_IMAGE_MODELS") {
            Ok(v) if !v.trim().is_empty() => split_env_list(&v),
            _ => vec![env_or_required("LLM_IMAGE_MODEL")],
        };
        let llm_image_model = {
            let d = env_or("LLM_IMAGE_MODELS_DEFAULT", "");
            if d.trim().is_empty() {
                llm_image_models.first().cloned().unwrap_or_default()
            } else {
                d
            }
        };

        let llm_video_models = match env::var("LLM_VIDEO_MODELS") {
            Ok(v) if !v.trim().is_empty() => split_env_list(&v),
            _ => vec![env_or_required("LLM_VIDEO_MODEL")],
        };
        let llm_video_model = {
            let d = env_or("LLM_VIDEO_MODELS_DEFAULT", "");
            if d.trim().is_empty() {
                llm_video_models.first().cloned().unwrap_or_default()
            } else {
                d
            }
        };

        let llm_model = llm_text_model.clone();
        let llm_api_key = llm_text_api_key.clone();
        let llm_api_keys = llm_text_api_keys.clone();

        // 数据库配置：如果不配 DATABASE_URL，默认使用 SQLite
        let data_dir = env_or("AIPPT_DATA_DIR", "data");
        let database_url = env_or("DATABASE_URL", &format!("sqlite://{}/walioffice.db?mode=rwc", data_dir));

        Ok(Self {
            app_name: env_or("AIPPT_APP_NAME", "WaLiOffice"),
            host: env_or("AIPPT_HOST", "0.0.0.0"),
            port: env_or("AIPPT_PORT", "8000").parse().unwrap_or(8000),

            jwt_secret,
            jwt_expiry_hours: env_or("AIPPT_JWT_EXPIRY_HOURS", "24").parse().unwrap_or(24),
            x_api_auth_login_url: env_or(
                "WALIOFFICE_X_API_AUTH_LOGIN_URL",
                "https://x-api.itedus.cn/api/v1/auth/login",
            ),

            llm_base_url: llm_text_base_url.clone(),
            llm_api_key,
            llm_api_keys,
            llm_model,
            llm_text_base_url,
            llm_text_api_key,
            llm_text_api_keys,
            llm_text_model,
            llm_text_models,
            llm_image_base_url,
            llm_image_api_key,
            llm_image_api_keys,
            llm_image_model,
            llm_image_models,
            llm_video_base_url,
            llm_video_api_key,
            llm_video_api_keys,
            llm_video_model,
            llm_video_models,
            llm_provider: env_or("AIPPT_LLM_PROVIDER", "glm-gateway"),
            llm_tool_timeout_ms: env_or("AIPPT_LLM_TOOL_TIMEOUT_MS", "1800000")
                .parse()
                .unwrap_or(1_800_000),
            llm_chat_timeout_ms: env_or("AIPPT_LLM_CHAT_TIMEOUT_MS", "1800000")
                .parse()
                .unwrap_or(1_800_000),
            web_search_provider: env_or("AIPPT_WEB_SEARCH_PROVIDER", "auto"),
            web_search_endpoint: env_or("AIPPT_WEB_SEARCH_ENDPOINT", "http://127.0.0.1:8080"),
            web_search_timeout_ms: env_or("AIPPT_WEB_SEARCH_TIMEOUT_MS", "20000")
                .parse()
                .unwrap_or(20_000),
            baidu_mcp_api_key: env_or("AIPPT_BAIDU_MCP_API_KEY", ""),
            baidu_mcp_sse_endpoint: env_or(
                "AIPPT_BAIDU_MCP_SSE_ENDPOINT",
                "http://appbuilder.baidu.com/v2/ai_search/mcp/sse",
            ),

            data_dir: data_dir.clone(),
            projects_dir: env_or("AIPPT_PROJECTS_DIR", "data/projects"),
            sessions_dir: env_or("AIPPT_SESSIONS_DIR", "data/sessions"),
            render_output_dir: env_or("AIPPT_RENDER_OUTPUT_DIR", "outputs"),

            cors_origins: env_or("AIPPT_CORS_ORIGINS", "http://localhost:5173")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),

            database_url,
            db_max_connections: env_or("DB_MAX_CONNECTIONS", "8").parse().unwrap_or(8),
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

    /// 判断是否使用 MySQL
    pub fn is_mysql(&self) -> bool {
        self.database_url.starts_with("mysql://")
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn split_api_keys(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn split_env_list(value: &str) -> Vec<String> {
    value
        .split(|ch| matches!(ch, ',' | ';' | '\n'))
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
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
