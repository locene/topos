use dotenv::from_filename;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct AppConfig {
    pub log_level: String,
    pub trow_url: String,
    pub trow_client_id: String,
    pub trow_client_secret: String,
    pub meilisearch_url: String,
    pub meilisearch_key: String,
    pub redis_url: String,
    pub admin_token: String,
}

impl AppConfig {
    fn load_config() -> Result<Self, Box<dyn Error>> {
        let profile = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let env_files = vec![
            format!(".env.{}.local", profile),
            format!(".env.{}", profile),
            ".env.local".to_string(),
            ".env".to_string(),
        ];

        println!("Current directory: {:?}", env::current_dir()?);

        for file_name in env_files {
            if let Some(path) = Self::find_env_file(&file_name) {
                match from_filename(&path) {
                    Ok(_) => println!("Loaded environment variables from: {:?}", path),
                    Err(e) => println!("Failed to parse {:?}: {}", path, e),
                }
            }
        }

        let log_level =
            env::var("LOG_LEVEL").map_err(|_| "LOG_LEVEL environment variable not set")?;

        let trow_url = env::var("TROW_URL").map_err(|_| "TROW_URL environment variable not set")?;
        let trow_client_id = env::var("TROW_CLIENT_ID")
            .map_err(|_| "TROW_CLIENT_ID environment variable not set")?;
        let trow_client_secret = env::var("TROW_CLIENT_SECRET")
            .map_err(|_| "TROW_CLIENT_SECRET environment variable not set")?;

        let meilisearch_url = env::var("MEILISEARCH_URL")
            .map_err(|_| "MEILISEARCH_URL environment variable not set")?;
        let meilisearch_key = env::var("MEILISEARCH_KEY")
            .map_err(|_| "MEILISEARCH_KEY environment variable not set")?;

        let redis_url =
            env::var("REDIS_URL").map_err(|_| "REDIS_URL environment variable not set")?;

        let admin_token =
            env::var("ADMIN_TOKEN").map_err(|_| "ADMIN_TOKEN environment variable not set")?;

        Ok(AppConfig {
            log_level,
            trow_url,
            trow_client_id,
            trow_client_secret,
            meilisearch_url,
            meilisearch_key,
            redis_url,
            admin_token,
        })
    }

    fn find_env_file(name: &str) -> Option<PathBuf> {
        let mut curr = env::current_dir().ok()?;

        loop {
            let candidate = curr.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
            if let Some(parent) = curr.parent() {
                curr = parent.to_path_buf();
            } else {
                break;
            }

            if curr.ends_with("src") || curr.parent().is_none() {
                let root_candidate = curr.parent().unwrap_or(&curr).join(name);
                if root_candidate.exists() {
                    return Some(root_candidate);
                }
                break;
            }
        }
        None
    }
}

pub static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn get_app_config() -> &'static AppConfig {
    APP_CONFIG.get_or_init(|| {
        AppConfig::load_config().expect("Failed to load application configuration at startup")
    })
}
