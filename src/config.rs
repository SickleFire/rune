use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuneConfig {
    pub provider: Option<String>,
    pub gemini_api_key: Option<String>,
    pub gemini_model: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: Option<String>,
    pub ollama_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub lm_studio_base_url: Option<String>,
    pub lm_studio_model: Option<String>,
    pub architect_model: Option<String>,
    pub coder_model: Option<String>,
}

impl Default for RuneConfig {
    fn default() -> Self {
        Self {
            provider: None,
            gemini_api_key: None,
            gemini_model: Some("gemini-3.5-flash-lite".to_string()),
            openai_api_key: None,
            openai_model: Some("gpt-5.6-luna".to_string()),
            anthropic_api_key: None,
            anthropic_model: Some("claude-3-5-sonnet".to_string()),
            ollama_base_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3".to_string()),
            lm_studio_base_url: Some("http://localhost:1234/v1".to_string()),
            lm_studio_model: Some("local-model".to_string()),
            architect_model: None,
            coder_model: None,
        }
    }
}

impl RuneConfig {
    /// Load configuration from `rune.toml` in the current working directory, project root,
    /// or home directory, with environment variable fallbacks.
    pub fn load() -> Self {
        let mut builder = config::Config::builder();

        // Start with default values
        let default_cfg = RuneConfig::default();
        if let Ok(default_val) = config::Config::try_from(&default_cfg) {
            builder = builder.add_source(default_val);
        }

        // Search for rune.toml in current directory or parent directories / home
        let config_filename = "rune.toml";
        let mut found_path: Option<PathBuf> = None;

        // Check current dir and parents
        if let Ok(current_dir) = env::current_dir() {
            let mut dir = Some(current_dir.as_path());
            while let Some(d) = dir {
                let candidate = d.join(config_filename);
                if candidate.exists() {
                    found_path = Some(candidate);
                    break;
                }
                dir = d.parent();
            }
        }

        // If not found, check home directory
        if found_path.is_none() {
            if let Some(home) = dirs_or_home() {
                let candidate = home.join(config_filename);
                if candidate.exists() {
                    found_path = Some(candidate);
                }
            }
        }

        if let Some(path) = found_path {
            println!("[Rune: Loaded configuration from {}]", path.display());
            builder = builder.add_source(config::File::from(path));
        }

        // Add environment variables with prefix RUNE (e.g. RUNE_GEMINI_API_KEY) or standard API keys
        // We can also allow standard env vars like GEMINI_API_KEY, OPENAI_API_KEY directly.
        builder = builder.add_source(
            config::Environment::with_prefix("RUNE")
                .separator("__")
                .ignore_empty(true),
        );

        let mut config = match builder.build() {
            Ok(c) => match c.try_deserialize::<RuneConfig>() {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("[Rune: Warning: Failed to parse rune.toml: {e}. Using defaults/env.]");
                    RuneConfig::default()
                }
            },
            Err(_) => RuneConfig::default(),
        };

        // Fallback / bridge direct standard environment variables if not set in TOML or RUNE_ env vars
        if config.gemini_api_key.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("GEMINI_API_KEY") {
                if !val.is_empty() {
                    config.gemini_api_key = Some(val);
                }
            }
        }
        if config.gemini_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("GEMINI_MODEL") {
                if !val.is_empty() {
                    config.gemini_model = Some(val);
                }
            }
        }

        if config.openai_api_key.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("OPENAI_API_KEY") {
                if !val.is_empty() {
                    config.openai_api_key = Some(val);
                }
            }
        }
        if config.openai_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("OPENAI_MODEL") {
                if !val.is_empty() {
                    config.openai_model = Some(val);
                }
            }
        }

        if config.anthropic_api_key.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("ANTHROPIC_API_KEY") {
                if !val.is_empty() {
                    config.anthropic_api_key = Some(val);
                }
            }
        }
        if config.anthropic_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("ANTHROPIC_MODEL") {
                if !val.is_empty() {
                    config.anthropic_model = Some(val);
                }
            }
        }

        if config.architect_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("ARCHITECT_MODEL") {
                if !val.is_empty() {
                    config.architect_model = Some(val);
                }
            }
        }
        if config.coder_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("CODER_MODEL") {
                if !val.is_empty() {
                    config.coder_model = Some(val);
                }
            }
        }

        config
    }
}

fn dirs_or_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
