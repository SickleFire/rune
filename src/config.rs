use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuneConfig {
    pub provider: Option<String>,
    pub gemini_api_key: Option<String>,
    pub gemini_model: Option<String>,
    pub gemini_min_request_interval_ms: u64,
    pub gemini_max_input_tokens_per_minute: Option<usize>,
    pub gemini_cascade_models: Option<Vec<String>>,
    pub enable_gemini_cascade: bool,
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub groq_api_key: Option<String>,
    pub groq_model: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: Option<String>,
    pub ollama_base_url: Option<String>,
    pub ollama_model: Option<String>,
    pub lm_studio_base_url: Option<String>,
    pub lm_studio_model: Option<String>,
    pub architect_model: Option<String>,
    pub coder_model: Option<String>,
    pub mysql_url: Option<String>,
    #[serde(default)]
    pub excluded_read_paths: Vec<String>,
    #[serde(default = "default_python_interpreter")]
    pub python_interpreter: String,
    #[serde(default)]
    pub python_args: Vec<String>,
    #[serde(default = "default_automation_timeout")]
    pub automation_timeout_seconds: u64,
    #[serde(default = "default_automation_max_output")]
    pub automation_max_output_bytes: usize,
    #[serde(default)]
    pub automations: Vec<crate::automation::AutomationConfig>,
}

impl Default for RuneConfig {
    fn default() -> Self {
        Self {
            provider: None,
            gemini_api_key: None,
            gemini_model: Some("gemini-3.5-flash-lite".to_string()),
            gemini_min_request_interval_ms: 1000,
            gemini_max_input_tokens_per_minute: None,
            gemini_cascade_models: Some(vec![
                "gemini-3.6-flash".to_string(),
                "gemini-3.8-flash".to_string(),
                "gemini-3.1-flash-lite".to_string(),
            ]),
            enable_gemini_cascade: false,
            openai_api_key: None,
            openai_model: Some("gpt-5.6-luna".to_string()),
            groq_api_key: None,
            groq_model: Some("openai/gpt-oss-120b".to_string()),
            openrouter_api_key: None,
            openrouter_model: None,
            anthropic_api_key: None,
            anthropic_model: Some("claude-3-5-sonnet".to_string()),
            ollama_base_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3".to_string()),
            lm_studio_base_url: Some("http://localhost:1234/v1".to_string()),
            lm_studio_model: Some("local-model".to_string()),
            architect_model: None,
            coder_model: None,
            mysql_url: None,
            excluded_read_paths: Vec::new(),
            python_interpreter: default_python_interpreter(),
            python_args: Vec::new(),
            automation_timeout_seconds: 30,
            automation_max_output_bytes: 8192,
            automations: Vec::new(),
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
                    found_path = Some(candidate.to_path_buf());
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
                    eprintln!(
                        "[Rune: Warning: Failed to parse rune.toml: {e}. Using defaults/env.]"
                    );
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

        if config.groq_api_key.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("GROQ_API_KEY") {
                if !val.is_empty() {
                    config.groq_api_key = Some(val);
                }
            }
        }
        if config.groq_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("GROQ_MODEL") {
                if !val.is_empty() {
                    config.groq_model = Some(val);
                }
            }
        }

        if config
            .openrouter_api_key
            .as_deref()
            .unwrap_or("")
            .is_empty()
        {
            if let Ok(val) = env::var("OPENROUTER_API_KEY") {
                if !val.is_empty() {
                    config.openrouter_api_key = Some(val);
                }
            }
        }
        if config.openrouter_model.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("OPENROUTER_MODEL") {
                if !val.is_empty() {
                    config.openrouter_model = Some(val);
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
        if config.mysql_url.as_deref().unwrap_or("").is_empty() {
            if let Ok(val) = env::var("MYSQL_URL") {
                if !val.is_empty() {
                    config.mysql_url = Some(val);
                }
            }
        }

        config
    }
}

fn default_python_interpreter() -> String {
    if cfg!(windows) {
        "py".to_string()
    } else {
        "python3".to_string()
    }
}

fn default_automation_timeout() -> u64 {
    30
}

fn default_automation_max_output() -> usize {
    8192
}

fn dirs_or_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_config_default() {
        let config = RuneConfig::default();
        assert_eq!(
            config.gemini_model.as_deref(),
            Some("gemini-3.5-flash-lite")
        );
        assert!(config.gemini_api_key.is_none());
        assert_eq!(config.gemini_min_request_interval_ms, 1000);
    }
}
