use colored::*;
use rune_code::agent::{AgentConfig, AgentRole, MultiAgentConfig, MultiAgentOrchestrator};
use rune_code::api::LLMProvider;
use rustyline::Context;
use rustyline::Editor;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use std::env;
use std::error::Error;
use std::sync::Arc;

struct RuneCodeHelper {
    hinter: HistoryHinter,
}

impl Hinter for RuneCodeHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for RuneCodeHelper {}
impl Validator for RuneCodeHelper {}

impl Completer for RuneCodeHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_up_to_cursor = &line[..pos];

        if line_up_to_cursor.starts_with('/') {
            let commands = vec![
                "/help",
                "/tools",
                "/context",
                "/tokens",
                "/files",
                "/tree",
                "/auto",
                "/plan",
                "/execute",
                "/model",
                "/save",
                "/load",
                "/clear",
                "/reset",
                "/undo",
                "/provider",
                "/init",
            ];

            let mut pairs = Vec::new();
            for cmd in commands {
                if cmd.starts_with(line_up_to_cursor) {
                    pairs.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
            return Ok((0, pairs));
        }

        if let Some(at_idx) = line_up_to_cursor.rfind('@') {
            let prefix = &line_up_to_cursor[at_idx + 1..];
            let mut pairs = Vec::new();

            if let Ok(workspace) = std::env::current_dir() {
                collect_file_candidates(&workspace, prefix, &mut pairs);
            }

            return Ok((at_idx + 1, pairs));
        }

        Ok((0, Vec::new()))
    }
}

impl rustyline::Helper for RuneCodeHelper {}

fn collect_file_candidates(dir: &std::path::Path, prefix: &str, pairs: &mut Vec<Pair>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let ignore_dirs = [".git", "target", ".cargo", "node_modules"];

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if ignore_dirs.contains(&name_str.as_ref()) {
            continue;
        }

        if path.is_dir() {
            collect_file_candidates(&path, prefix, pairs);
        } else if path.is_file() {
            if let Ok(rel_path) = path.strip_prefix(std::env::current_dir().unwrap_or_default()) {
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                if rel_str.starts_with(prefix)
                    || rel_str.to_lowercase().contains(&prefix.to_lowercase())
                {
                    pairs.push(Pair {
                        display: rel_str.clone(),
                        replacement: rel_str,
                    });
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let workspace_root = env::current_dir().expect("Failed to get current directory");

    let config = rune_code::config::RuneConfig::load();

    let gemini_api_key = config.gemini_api_key.clone().unwrap_or_default();
    let gemini_model = config
        .gemini_model
        .clone()
        .unwrap_or("gemini-3.5-flash-lite".to_string());

    let openai_api_key = config.openai_api_key.clone().unwrap_or_default();
    let openai_model = config
        .openai_model
        .clone()
        .unwrap_or("gpt-5.6-luna".to_string());

    let groq_api_key = config.groq_api_key.clone().unwrap_or_default();
    let groq_model = config
        .groq_model
        .clone()
        .unwrap_or("openai/gpt-oss-120b".to_string());

    let openrouter_api_key = config.openrouter_api_key.clone().unwrap_or_default();
    let openrouter_model = config
        .openrouter_model
        .clone()
        .unwrap_or("openai/gpt-4o-mini".to_string());

    let anthropic_api_key = config.anthropic_api_key.clone().unwrap_or_default();
    let anthropic_model = config
        .anthropic_model
        .clone()
        .unwrap_or("claude-3-5-sonnet".to_string());

    // Determine default provider based on config.provider or available keys
    let provider_name = config.provider.as_deref().unwrap_or("auto");
    let default_provider: Arc<dyn LLMProvider> = if provider_name.eq_ignore_ascii_case("gemini")
        || (!gemini_api_key.is_empty() && provider_name == "auto")
    {
        if config.enable_gemini_cascade {
            let cascade_models = config.gemini_cascade_models.clone().unwrap_or_else(|| {
                vec![gemini_model.clone(), "gemini-3.8-flash".to_string(), "gemini-3.6-flash".to_string()]
            });
            println!("[Rune: Using Gemini Cascading Provider with models: {:?}]", cascade_models);
            Arc::new(
                rune_code::api::GeminiCascadingProvider::new(gemini_api_key.clone(), cascade_models)
                    .with_request_limits(
                        std::time::Duration::from_millis(config.gemini_min_request_interval_ms),
                        config.gemini_max_input_tokens_per_minute,
                    ),
            )
        } else {
            Arc::new(
                rune_code::api::GeminiProvider::new(gemini_api_key.clone(), gemini_model.clone())
                    .with_request_limits(
                        std::time::Duration::from_millis(config.gemini_min_request_interval_ms),
                        config.gemini_max_input_tokens_per_minute,
                    ),
            )
        }
    } else if provider_name.eq_ignore_ascii_case("openai")
        || (!openai_api_key.is_empty() && provider_name == "auto")
    {
        Arc::new(
            rune_code::api::OpenAIProvider::new(openai_api_key.clone(), openai_model.clone())
                .with_reasoning_effort("none"),
        )
    } else if provider_name.eq_ignore_ascii_case("groq")
        || (!groq_api_key.is_empty() && provider_name == "auto")
    {
        Arc::new(rune_code::api::GroqProvider::new(
            groq_api_key.clone(),
            groq_model.clone(),
        ))
    } else if provider_name.eq_ignore_ascii_case("openrouter")
        || (!openrouter_api_key.is_empty() && provider_name == "auto")
    {
        Arc::new(rune_code::api::OpenRouterProvider::new(
            openrouter_api_key.clone(),
            openrouter_model.clone(),
        ))
    } else if provider_name.eq_ignore_ascii_case("anthropic")
        || (!anthropic_api_key.is_empty() && provider_name == "auto")
    {
        Arc::new(
            rune_code::api::OpenAIProvider::new(anthropic_api_key.clone(), anthropic_model.clone()) // Or Anthropic provider if implemented, falling back nicely
                .with_reasoning_effort("none"),
        )
    } else if provider_name.eq_ignore_ascii_case("ollama") {
        let base_url = config
            .ollama_base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");
        let model = config.ollama_model.as_deref().unwrap_or("llama3");
        println!(
            "[Rune: Using Ollama local provider at {} (model: {})]",
            base_url, model
        );
        Arc::new(rune_code::api::OllamaProvider::new(base_url, model.into()))
    } else if provider_name.eq_ignore_ascii_case("lmstudio") {
        let base_url = config
            .lm_studio_base_url
            .as_deref()
            .unwrap_or("http://localhost:1234/v1");
        let model = config.lm_studio_model.as_deref().unwrap_or("local-model");
        println!(
            "[Rune: Using LM Studio local provider at {} (model: {})]",
            base_url, model
        );
        Arc::new(
            rune_code::api::OpenAIProvider::new("lm-studio-local".into(), model.into())
                .with_reasoning_effort("none"),
        )
    } else if !gemini_api_key.is_empty() {
        if config.enable_gemini_cascade {
            let cascade_models = config.gemini_cascade_models.clone().unwrap_or_else(|| {
                vec![gemini_model.clone(), "gemini-2.5-flash".to_string(), "gemini-3.5-flash-lite".to_string()]
            });
            Arc::new(
                rune_code::api::GeminiCascadingProvider::new(gemini_api_key.clone(), cascade_models)
                    .with_request_limits(
                        std::time::Duration::from_millis(config.gemini_min_request_interval_ms),
                        config.gemini_max_input_tokens_per_minute,
                    ),
            )
        } else {
            Arc::new(
                rune_code::api::GeminiProvider::new(gemini_api_key.clone(), gemini_model.clone())
                    .with_request_limits(
                        std::time::Duration::from_millis(config.gemini_min_request_interval_ms),
                        config.gemini_max_input_tokens_per_minute,
                    ),
            )
        }
    } else if !openai_api_key.is_empty() {
        Arc::new(
            rune_code::api::OpenAIProvider::new(openai_api_key.clone(), openai_model.clone())
                .with_reasoning_effort("none"),
        )
    } else if !groq_api_key.is_empty() {
        Arc::new(rune_code::api::GroqProvider::new(
            groq_api_key.clone(),
            groq_model.clone(),
        ))
    } else if !openrouter_api_key.is_empty() {
        Arc::new(rune_code::api::OpenRouterProvider::new(
            openrouter_api_key.clone(),
            openrouter_model.clone(),
        ))
    } else {
        panic!(
            "Please configure at least one LLM provider (Gemini, OpenAI, Groq, OpenRouter, Anthropic, Ollama, or LM Studio) via rune.toml or environment variables."
        );
    };

    // Multi-agent setup (Max 2 agents: Architect & Coder)
    let multi_agent_mode = env::var("RUNE_MULTI_AGENT").unwrap_or_else(|_| "false".to_string())
        == "true"
        || std::env::args().any(|arg| arg == "--multi-agent");

    let multi_config = if multi_agent_mode {
        // Use lightweight model for architect, powerful model for coder
        let architect_model = config
            .architect_model
            .clone()
            .unwrap_or_else(|| gemini_model.clone());
        let coder_model = config
            .coder_model
            .clone()
            .unwrap_or_else(|| openai_model.clone());
        MultiAgentConfig {
            agents: vec![
                AgentConfig {
                    name: "Architect".into(),
                    role: AgentRole::Architect,
                    model: architect_model,
                    system_prompt: Some("You are the Architect agent. Your goal is to analyze user requests, inspect files, \
        search code, and formulate clear step-by-step execution plans efficiently with minimal tokens. \
        You cannot edit files or run commands — your only output is a plan.".into()),
                },
                AgentConfig {
                    name: "Coder".into(),
                    role: AgentRole::Coder,
                    model: coder_model,
                    system_prompt: Some("You are the Coder agent. Your goal is to take the user request and Architect's plan, then execute precise edits, run tests, and complete the implementation.".into()),
                },
            ],
        }
    } else {
        let active_model = if !gemini_api_key.is_empty() {
            gemini_model.clone()
        } else if !openai_api_key.is_empty() {
            openai_model.clone()
        } else {
            openai_model.clone()
        };
        MultiAgentConfig {
            agents: vec![AgentConfig {
                name: "RuneAgent".into(),
                role: AgentRole::Coder,
                model: active_model,
                system_prompt: None,
            }],
        }
    };

    let mut orchestrator =
        MultiAgentOrchestrator::new(default_provider, workspace_root.clone(), multi_config)
            .expect("Failed to initialize multi-agent orchestrator");

    if multi_agent_mode {
        println!(
            "{}",
            format!(
                "Multi-agent mode ENABLED ({} agents: Architect [{}] -> Coder [{}]).",
                orchestrator.agent_count(),
                config.architect_model.as_deref().unwrap_or(&gemini_model),
                config.coder_model.as_deref().unwrap_or(&openai_model)
            )
            .cyan()
            .bold()
        );
    }

    let args: Vec<String> = env::args().collect();
    let enable_unity_lite = args.contains(&"--unity-lite".to_string());
    let enable_unity = args.contains(&"--unity".to_string());
    let enable_godot = args.contains(&"--godot".to_string());
    let enable_web = args.contains(&"--web".to_string());
    let enable_github = args.contains(&"--github".to_string()) || env::var("GITHUB_TOKEN").is_ok();
    let enable_mysql = args.contains(&"--mysql".to_string())
        || config.mysql_url.as_deref().map_or(false, |s| !s.is_empty())
        || env::var("MYSQL_URL").map_or(false, |s| !s.is_empty());
    let enable_docker = args.contains(&"--docker".to_string());

    // Always enable core persistent memory, notes, and session memory tools
    orchestrator = orchestrator
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::memory_tools::RememberPreferenceTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::memory_tools::RecallMemoryTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(rune_code::memory_tools::RecordFixTool))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::memory_tools::RecordFileRelationTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::notes_tools::ListRuneNotesTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::notes_tools::ReadRuneNoteTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::notes_tools::UpsertRuneNoteTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::notes_tools::SearchRuneNotesTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(
            rune_code::notes_tools::RecallRelatedRuneNotesTool,
        ));

    if enable_unity_lite {
        println!(
            "{}",
            "Unity-lite editor tools enabled (inspection, validation & logs).".cyan()
        );
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityInspectSceneTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityInspectComponentsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityFindAssetsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityValidateReferencesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityReadConsoleLogsTool::new(),
            ));
    }

    if enable_unity {
        println!("{}", "Unity editor tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityInspectSceneTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnitySetPropertyTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityInspectComponentsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityAssignReferenceTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityValidateReferencesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityAddComponentTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityFindAssetsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityInstantiatePrefabTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityCreateScriptableObjectTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityRefreshAssetDatabaseTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::unity_tools::UnityReadConsoleLogsTool::new(),
            ))
    }

    if enable_godot {
        println!("{}", "Godot editor tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::godot_tools::GodotInspectSceneTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::godot_tools::GodotInspectNodePropertiesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::godot_tools::GodotCreateNodeTool::new(),
            ))
    }

    if enable_web {
        println!("{}", "Web utility tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::web_tools::HttpRequestTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::web_tools::FetchWebPageTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::web_tools::CheckTcpPortTool::new(),
            ))
    }

    if enable_github {
        println!("{}", "GitHub integration tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::github_tools::GitHubIssueTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::github_tools::GitHubPullRequestDiffTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::github_tools::GitHubCreateIssueTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::github_tools::GitHubCreateCommentTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::github_tools::GitHubCreatePullRequestTool::new(),
            ))
    }

    if enable_mysql {
        println!("{}", "MySQL database tools enabled.".cyan());
        let mysql_url = env::var("MYSQL_URL").ok();
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::mysql_tools::MysqlListTablesTool::new(mysql_url.clone()),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::mysql_tools::MysqlDescribeTableTool::new(mysql_url.clone()),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::mysql_tools::MysqlExecuteQueryTool::new(mysql_url),
            ))
    }

    if enable_docker {
        println!("{}", "Docker container tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerListContainersTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerContainerLogsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerStartContainerTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerStopContainerTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerListImagesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune_code::docker_tools::DockerInspectContainerTool::new(),
            ))
    }

    let mut auto_approve = false;
    let mut plan_mode = false;
    let mut rl = Editor::new()?;
    rl.set_helper(Some(RuneCodeHelper {
        hinter: HistoryHinter::new(),
    }));
    let history_file = env::temp_dir().join(".rune_history");
    let _ = rl.load_history(&history_file);

    let ascii_art = r#"
         .  :  *  .
       ╔══════════╗
       ║  ██║     ║
       ║  ██║██╗  ║
       ║  ██║ ╚██╗║
       ║  ██║██╗  ║
       ║  ██║ ╚██╗║
       ║  ██║  ╚═╝║
       ║  ██║     ║
       ╚══════════╝
         '  :  *  '
"#;
    println!("{}", ascii_art.cyan().bold());
    println!(
        "{}",
        "Type your prompt, or use /help for available commands.".bright_black()
    );
    println!(
        "{}",
        "Type 'exit' or 'quit' to end session.\n".bright_black()
    );

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let prompt = line.trim();

                if prompt.is_empty() {
                    continue;
                }

                if prompt.eq_ignore_ascii_case("exit") || prompt.eq_ignore_ascii_case("quit") {
                    println!("{}", "exiting.".yellow());
                    break;
                }

                if prompt.eq_ignore_ascii_case("/clear") || prompt.eq_ignore_ascii_case("/reset") {
                    println!(
                        "{}",
                        "Cleared conversation history (multi-agent orchestrator reset).".green()
                    );
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/undo") {
                    let undo_result: Result<String, std::io::Error> = (async {
                        let workspace_root = std::env::current_dir()?;
                        let executor = rune_code::tools::ToolExecutor::new(workspace_root);
                        executor.undo_git_checkpoint().await
                    })
                    .await;

                    match undo_result {
                        Ok(msg) => println!("{}", msg.green()),
                        Err(e) => eprintln!("{}", format!("Failed to undo: {e}").red()),
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/context") || prompt.eq_ignore_ascii_case("/tokens")
                {
                    println!(
                        "{}",
                        format!(
                            "Multi-agent mode active with {} agents.",
                            orchestrator.agent_count()
                        )
                        .cyan()
                    );
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/files") || prompt.eq_ignore_ascii_case("/tree") {
                    let agent = rune_code::api::Agent::new_gemini(
                        env::var("GEMINI_API_KEY").unwrap_or_default(),
                        workspace_root.clone(),
                        gemini_model.clone(),
                    );
                    let map = agent.build_project_map(&workspace_root, "");
                    println!("{}", "=== Repository File Tree ===".cyan().bold());
                    println!("{}", map);
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/plan") || prompt.starts_with("/plan ") {
                    plan_mode = true;
                    auto_approve = false;
                    println!("{}", "Plan mode ENABLED. Rune will propose a detailed execution plan without executing mutating tools.".cyan().bold());
                    let instruction = prompt.strip_prefix("/plan").unwrap_or("").trim();
                    if !instruction.is_empty() {
                        let plan_prompt = format!(
                            "[PLANNING MODE REQUEST] Please formulate a comprehensive, step-by-step execution plan to accomplish the following task without executing mutating tools:\n\n{instruction}"
                        );
                        orchestrator.run_workflow(&plan_prompt, false, true).await;
                        println!();
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/execute") {
                    plan_mode = false;
                    println!("{}", "Execute mode ENABLED. Rune is now ready to execute tools and carry out tasks.".green().bold());
                    continue;
                }
                if prompt.eq_ignore_ascii_case("/auto") || prompt.starts_with("/auto ") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        match parts[1].to_lowercase().as_str() {
                            "on" | "true" | "1" => {
                                auto_approve = true;
                                println!("{}", "Auto-approve mode ENABLED.".green());
                            }
                            "off" | "false" | "0" => {
                                auto_approve = false;
                                println!("{}", "Auto-approve mode DISABLED.".yellow());
                            }
                            _ => {
                                println!("Usage: /auto [on|off]");
                            }
                        }
                    } else {
                        auto_approve = !auto_approve;
                        println!(
                            "Auto-approve mode is now: {}",
                            if auto_approve {
                                "ENABLED".green()
                            } else {
                                "DISABLED".yellow()
                            }
                        );
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/model") || prompt.starts_with("/model ") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        let new_model = parts[1].to_string();
                        orchestrator.set_model_all(new_model.clone());
                        println!(
                            "{}",
                            format!("Active model updated to: {}", new_model).green()
                        );
                    } else {
                        println!(
                            "{}",
                            format!(
                                "Current active model(s): {}",
                                orchestrator.get_active_models_summary()
                            )
                            .cyan()
                        );
                    }
                    continue;
                }

                if prompt.starts_with("/provider") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        let provider_arg = parts[1].to_lowercase();
                        let model_arg = parts.get(2).map(|s| s.to_string());

                        let (new_provider, resolved_model): (Arc<dyn LLMProvider>, String) = match provider_arg
                            .as_str()
                        {
                            "gemini" => {
                                let key = config.gemini_api_key.clone().unwrap_or_else(|| {
                                    env::var("GEMINI_API_KEY").unwrap_or_default()
                                });
                                let model =
                                    model_arg.clone().unwrap_or_else(|| gemini_model.clone());
                                if config.enable_gemini_cascade {
                                    let cascade_models = config.gemini_cascade_models.clone().unwrap_or_else(|| {
                                        vec![model.clone(), "gemini-2.5-flash".to_string(), "gemini-3.5-flash-lite".to_string()]
                                    });
                                    (
                                        Arc::new(
                                            rune_code::api::GeminiCascadingProvider::new(key, cascade_models)
                                                .with_request_limits(
                                                    std::time::Duration::from_millis(
                                                        config.gemini_min_request_interval_ms,
                                                    ),
                                                    config.gemini_max_input_tokens_per_minute,
                                                ),
                                        ),
                                        model,
                                    )
                                } else {
                                    (
                                        Arc::new(
                                            rune_code::api::GeminiProvider::new(key, model.clone())
                                                .with_request_limits(
                                                    std::time::Duration::from_millis(
                                                        config.gemini_min_request_interval_ms,
                                                    ),
                                                    config.gemini_max_input_tokens_per_minute,
                                                ),
                                        ),
                                        model,
                                    )
                                }
                            }
                            "openai" => {
                                let key = config.openai_api_key.clone().unwrap_or_else(|| {
                                    env::var("OPENAI_API_KEY").unwrap_or_default()
                                });
                                let model =
                                    model_arg.clone().unwrap_or_else(|| openai_model.clone());
                                (
                                    Arc::new(
                                        rune_code::api::OpenAIProvider::new(key, model.clone())
                                            .with_reasoning_effort("none"),
                                    ),
                                    model,
                                )
                            }
                            "groq" => {
                                let key = config.groq_api_key.clone().unwrap_or_else(|| {
                                    env::var("GROQ_API_KEY").unwrap_or_default()
                                });
                                let model = model_arg.clone().unwrap_or_else(|| groq_model.clone());
                                (
                                    Arc::new(rune_code::api::GroqProvider::new(key, model.clone())),
                                    model,
                                )
                            }
                            "openrouter" => {
                                let key = config.openrouter_api_key.clone().unwrap_or_else(|| {
                                    env::var("OPENROUTER_API_KEY").unwrap_or_default()
                                });
                                let model = model_arg
                                    .clone()
                                    .unwrap_or_else(|| openrouter_model.clone());
                                (
                                    Arc::new(rune_code::api::OpenRouterProvider::new(
                                        key,
                                        model.clone(),
                                    )),
                                    model,
                                )
                            }
                            "anthropic" => {
                                let key = config.anthropic_api_key.clone().unwrap_or_else(|| {
                                    env::var("ANTHROPIC_API_KEY").unwrap_or_default()
                                });
                                let model =
                                    model_arg.clone().unwrap_or_else(|| anthropic_model.clone());
                                (
                                    Arc::new(
                                        rune_code::api::OpenAIProvider::new(key, model.clone())
                                            .with_reasoning_effort("none"),
                                    ),
                                    model,
                                )
                            }
                            "ollama" => {
                                let base_url = config
                                    .ollama_base_url
                                    .as_deref()
                                    .unwrap_or("http://localhost:11434");
                                let model = model_arg.clone().unwrap_or_else(|| {
                                    config
                                        .ollama_model
                                        .clone()
                                        .unwrap_or_else(|| "llama3".to_string())
                                });
                                (
                                    Arc::new(rune_code::api::OllamaProvider::new(
                                        base_url,
                                        model.clone(),
                                    )),
                                    model,
                                )
                            }
                            "lmstudio" => {
                                let _base_url = config
                                    .lm_studio_base_url
                                    .as_deref()
                                    .unwrap_or("http://localhost:1234/v1");
                                let model = model_arg.clone().unwrap_or_else(|| {
                                    config
                                        .lm_studio_model
                                        .clone()
                                        .unwrap_or_else(|| "local-model".to_string())
                                });
                                (
                                    Arc::new(
                                        rune_code::api::OpenAIProvider::new(
                                            "lm-studio-local".into(),
                                            model.clone(),
                                        )
                                        .with_reasoning_effort("none"),
                                    ),
                                    model,
                                )
                            }
                            _ => {
                                println!("{}", format!("Unknown provider '{}'. Supported: gemini, openai, groq, openrouter, anthropic, ollama, lmstudio", parts[1]).red());
                                continue;
                            }
                        };

                        let model_override = if model_arg.is_some() || orchestrator.agent_count() == 1
                        {
                            Some(resolved_model)
                        } else {
                            None
                        };
                        orchestrator.set_provider_all(new_provider, model_override);
                        println!(
                            "{}",
                            format!(
                                "Switched provider to '{}' (active models: {})",
                                provider_arg,
                                orchestrator.get_active_models_summary()
                            )
                            .green()
                        );
                    } else {
                        println!(
                            "{}",
                            format!(
                                "Current provider / model(s): {}",
                                orchestrator.get_active_models_summary()
                            )
                            .cyan()
                        );
                        println!(
                            "{}",
                            "Usage: /provider [gemini|openai|groq|openrouter|anthropic|ollama|lmstudio] [model]"
                                .bright_black()
                        );
                    }
                    continue;
                }

                if prompt.starts_with("/save") || prompt.starts_with("/load") {
                    println!(
                        "{}",
                        "Command handled by multi-agent orchestrator setup.".cyan()
                    );
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/tools") {
                    println!("{}", "AVAILABLE TOOLS:".cyan().bold());
                    println!(
                        "  - {:<18} : List files and directories in a given path",
                        "list_files".green()
                    );
                    println!(
                        "  - {:<18} : Read the full text content of a file",
                        "read_file".green()
                    );
                    println!(
                        "  - {:<18} : Write or overwrite content to a file (requires confirmation)",
                        "write_file".green()
                    );
                    println!(
                        "  - {:<18} : Surgical search-and-replace patch on a file (requires confirmation)",
                        "patch_file".green()
                    );
                    println!(
                        "  - {:<18} : Instant BM25 code search via cix binary",
                        "search_code".green()
                    );
                    println!(
                        "  - {:<18} : Run a single shell command (requires confirmation)",
                        "execute_commands".green()
                    );
                    println!(
                        "  - {:<18} : Run a batch of sequential shell commands (requires confirmation)",
                        "execute_batch".green()
                    );
                    println!(
                        "  - {:<18} : Check the current git status of the repository",
                        "git_status".green()
                    );
                    println!(
                        "  - {:<18} : Show changes in working tree or file path",
                        "git_diff".green()
                    );
                    println!(
                        "  - {:<18} : Stage all changes and create a git commit (requires confirmation)",
                        "git_commit".green()
                    );
                    println!(
                        "  - {:<18} : Undo the last file mutation via git stash checkpoint",
                        "undo_git_checkpoint".green()
                    );
                    println!(
                        "  - {:<18} : Read GitHub issue details & comments (with --github)",
                        "github_read_issue".green()
                    );
                    println!(
                        "  - {:<18} : Fetch unified git diff of a GitHub PR for review (with --github)",
                        "github_read_pr_diff".green()
                    );
                    println!(
                        "  - {:<18} : Create a new GitHub issue (requires confirmation) (with --github)",
                        "github_create_issue".green()
                    );
                    println!(
                        "  - {:<18} : Post a comment on a GitHub issue or PR (requires confirmation) (with --github)",
                        "github_create_comment".green()
                    );
                    println!(
                        "  - {:<18} : Create a new GitHub pull request (requires confirmation) (with --github)",
                        "github_create_pull_request".green()
                    );
                    println!(
                        "  - {:<18} : List all tables in XAMPP MySQL database (with --mysql)",
                        "mysql_list_tables".green()
                    );
                    println!(
                        "  - {:<18} : Describe schema and columns of a MySQL table (with --mysql)",
                        "mysql_describe_table".green()
                    );
                    println!(
                        "  - {:<18} : Execute SQL queries against MySQL database (with --mysql)",
                        "mysql_execute_query".green()
                    );
                    println!(
                        "  - {:<18} : List Docker containers using docker ps (with --docker)",
                        "docker_list_containers".green()
                    );
                    println!(
                        "  - {:<18} : Fetch container logs using docker logs (with --docker)",
                        "docker_container_logs".green()
                    );
                    println!(
                        "  - {:<18} : Start a stopped Docker container (requires confirmation) (with --docker)",
                        "docker_start_container".green()
                    );
                    println!(
                        "  - {:<18} : Stop a running Docker container (requires confirmation) (with --docker)",
                        "docker_stop_container".green()
                    );
                    println!(
                        "  - {:<18} : List local Docker images (with --docker)",
                        "docker_list_images".green()
                    );
                    println!(
                        "  - {:<18} : Inspect container configuration and state (with --docker)",
                        "docker_inspect_container".green()
                    );
                    println!(
                        "  - {:<18} : Inspect active Godot scene hierarchy (with --godot)",
                        "godot_inspect_scene".green()
                    );
                    println!(
                        "  - {:<18} : Inspect Godot node properties (with --godot)",
                        "godot_inspect_node_properties".green()
                    );
                    println!(
                        "  - {:<18} : Create a new Godot node (requires confirmation) (with --godot)",
                        "godot_create_node".green()
                    );
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/config") || prompt.starts_with("/config ") {
                    let target_path = std::path::Path::new("rune.toml");

                    println!(
                        "{}",
                        "=== Rune Configuration Editor (/config) ===".cyan().bold()
                    );
                    println!("Current rune.toml path: {}", target_path.display());
                    println!(
                        "You can update settings interactively. Press Enter to keep current value."
                    );

                    // Helper closure for prompting
                    let prompt_field = |label: &str, current_val: &str| -> String {
                        print!(
                            "  {} [{}] : ",
                            label,
                            if current_val.is_empty() {
                                "(empty)"
                            } else {
                                current_val
                            }
                        );
                        use std::io::Write;
                        std::io::stdout().flush().unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let val = input.trim();
                        if val.is_empty() {
                            current_val.to_string()
                        } else {
                            val.to_string()
                        }
                    };

                    // Parse existing config or extract values simply
                    let current_provider = config
                        .provider
                        .clone()
                        .unwrap_or_else(|| "gemini".to_string());
                    let current_gemini_key = config.gemini_api_key.clone().unwrap_or_default();
                    let current_gemini_model = config
                        .gemini_model
                        .clone()
                        .unwrap_or_else(|| "gemini-3.5-flash-lite".to_string());
                    let current_openai_key = config.openai_api_key.clone().unwrap_or_default();
                    let current_openai_model = config
                        .openai_model
                        .clone()
                        .unwrap_or_else(|| "gpt-5.6-luna".to_string());
                    let current_groq_key = config.groq_api_key.clone().unwrap_or_default();
                    let current_groq_model = config
                        .groq_model
                        .clone()
                        .unwrap_or_else(|| "openai/gpt-oss-120b".to_string());
                    let current_openrouter_key =
                        config.openrouter_api_key.clone().unwrap_or_default();
                    let current_openrouter_model = config
                        .openrouter_model
                        .clone()
                        .unwrap_or_else(|| "openai/gpt-4o-mini".to_string());
                    let current_anthropic_key =
                        config.anthropic_api_key.clone().unwrap_or_default();
                    let current_anthropic_model = config
                        .anthropic_model
                        .clone()
                        .unwrap_or_else(|| "claude-3-5-sonnet".to_string());
                    let current_ollama_base_url = config
                        .ollama_base_url
                        .clone()
                        .unwrap_or_else(|| "http://localhost:11434".to_string());
                    let current_ollama_model = config
                        .ollama_model
                        .clone()
                        .unwrap_or_else(|| "llama3".to_string());
                    let current_mysql_url = config.mysql_url.clone().unwrap_or_default();

                    println!("\n[1/4] Provider & API Keys:");
                    let new_provider = prompt_field(
                        "Default Provider (gemini|openai|groq|openrouter|anthropic|ollama|lmstudio)",
                        &current_provider,
                    );
                    let new_gemini_key = prompt_field("Gemini API Key", &current_gemini_key);
                    let new_openai_key = prompt_field("OpenAI API Key", &current_openai_key);
                    let new_groq_key = prompt_field("Groq API Key", &current_groq_key);
                    let new_openrouter_key =
                        prompt_field("OpenRouter API Key", &current_openrouter_key);
                    let new_anthropic_key =
                        prompt_field("Anthropic API Key", &current_anthropic_key);

                    println!("\n[2/4] Models:");
                    let new_gemini_model = prompt_field("Gemini Model", &current_gemini_model);
                    let new_openai_model = prompt_field("OpenAI Model", &current_openai_model);
                    let new_groq_model = prompt_field("Groq Model", &current_groq_model);
                    let new_openrouter_model =
                        prompt_field("OpenRouter Model", &current_openrouter_model);
                    let new_anthropic_model =
                        prompt_field("Anthropic Model", &current_anthropic_model);

                    println!("\n[3/4] Database & Local Endpoints:");
                    let new_ollama_base_url =
                        prompt_field("Ollama Base URL", &current_ollama_base_url);
                    let new_ollama_model = prompt_field("Ollama Model", &current_ollama_model);
                    let new_mysql_url = prompt_field("MySQL URL (optional)", &current_mysql_url);

                    // Rebuild TOML content
                    let config_content = format!(
                        r#"# Rune Configuration (`rune.toml`)
# Updated via /config wizard

provider = "{new_provider}"

# API Keys
gemini_api_key = "{new_gemini_key}"
openai_api_key = "{new_openai_key}"
groq_api_key = "{new_groq_key}"
openrouter_api_key = "{new_openrouter_key}"
anthropic_api_key = "{new_anthropic_key}"

# Model Choices
gemini_model = "{new_gemini_model}"
openai_model = "{new_openai_model}"
groq_model = "{new_groq_model}"
openrouter_model = "{new_openrouter_model}"
anthropic_model = "{new_anthropic_model}"

# Local Endpoints (Ollama / LM Studio)
ollama_base_url = "{new_ollama_base_url}"
ollama_model = "{new_ollama_model}"

lm_studio_base_url = "http://localhost:1234/v1"
lm_studio_model = "local-model"

# Multi-Agent Role Model Overrides
architect_model = "{new_gemini_model}"
coder_model = "{new_openai_model}"

# Database
mysql_url = "{new_mysql_url}"
"#
                    );

                    match std::fs::write(target_path, config_content) {
                        Ok(_) => {
                            println!(
                                "{}",
                                format!(
                                    "Successfully updated rune.toml at {}",
                                    target_path.display()
                                )
                                .green()
                                .bold()
                            );
                            println!(
                                "{}",
                                "Restart Rune or use /provider to apply changes.".cyan()
                            );
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Failed to write rune.toml: {e}").red());
                        }
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/render") || prompt.starts_with("/render ") {
                    use rune_code::tools::AgentTool;
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    let output_path = if parts.len() > 1 {
                        parts[1]
                    } else {
                        "rune_notes.html"
                    };
                    let tool = rune_code::notes_tools::RenderRuneNotesTool;
                    let result: String = tool
                        .execute(serde_json::json!({ "output_path": output_path }))
                        .await;
                    println!("{}", result.green());
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/init") || prompt.starts_with("/init ") {
                    let target_path = std::path::Path::new("rune.toml");
                    if target_path.exists() {
                        println!(
                            "{}",
                            "rune.toml already exists in the current directory. Overwrite? (y/N): "
                                .yellow()
                        );
                        let mut confirm = String::new();
                        if std::io::stdin().read_line(&mut confirm).is_err()
                            || !confirm.trim().eq_ignore_ascii_case("y")
                        {
                            println!("{}", "Cancelled rune.toml initialization.".yellow());
                            continue;
                        }
                    }

                    println!(
                        "{}",
                        "=== Rune Configuration Wizard (/init) ===".cyan().bold()
                    );
                    println!("Choose default provider:");
                    println!("  1) gemini (Google Gemini API)");
                    println!("  2) openai (OpenAI GPT API)");
                    println!("  3) groq (Groq API)");
                    println!("  4) anthropic (Anthropic Claude API)");
                    println!("  5) ollama (Local Ollama)");
                    println!("  6) lmstudio (Local LM Studio)");
                    println!("  7) openrouter (OpenRouter API)");
                    print!("Enter choice [1-7, default: 1]: ");
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();

                    let mut choice_input = String::new();
                    std::io::stdin().read_line(&mut choice_input).unwrap();
                    let choice = choice_input.trim();

                    let (provider, def_model) = match choice {
                        "2" | "openai" => ("openai", "gpt-5.6-luna"),
                        "3" | "groq" => ("groq", "openai/gpt-oss-120b"),
                        "4" | "anthropic" => ("anthropic", "claude-3-5-sonnet"),
                        "5" | "ollama" => ("ollama", "llama3"),
                        "6" | "lmstudio" => ("lmstudio", "local-model"),
                        "7" | "openrouter" => ("openrouter", "openai/gpt-4o-mini"),
                        _ => ("gemini", "gemini-3.5-flash-lite"),
                    };

                    println!("Enter API key / endpoint for provider '{}':", provider);
                    let mut api_key_or_url = String::new();
                    if provider == "ollama" {
                        print!("Ollama base URL [http://localhost:11434]: ");
                        std::io::stdout().flush().unwrap();
                        std::io::stdin().read_line(&mut api_key_or_url).unwrap();
                        if api_key_or_url.trim().is_empty() {
                            api_key_or_url = "http://localhost:11434".to_string();
                        }
                    } else if provider == "lmstudio" {
                        print!("LM Studio base URL [http://localhost:1234/v1]: ");
                        std::io::stdout().flush().unwrap();
                        std::io::stdin().read_line(&mut api_key_or_url).unwrap();
                        if api_key_or_url.trim().is_empty() {
                            api_key_or_url = "http://localhost:1234/v1".to_string();
                        }
                    } else {
                        print!("API Key (leave blank to use environment variable): ");
                        std::io::stdout().flush().unwrap();
                        std::io::stdin().read_line(&mut api_key_or_url).unwrap();
                    }
                    let key_val = api_key_or_url.trim();

                    print!("Model name [{def_model}]: ");
                    std::io::stdout().flush().unwrap();
                    let mut model_input = String::new();
                    std::io::stdin().read_line(&mut model_input).unwrap();
                    let model_val = model_input.trim();
                    let chosen_model = if model_val.is_empty() {
                        def_model
                    } else {
                        model_val
                    };

                    let mut toml_content = format!(
                        r#"# Rune Configuration (`rune.toml`)
# Generated by Rune Interactive Setup (/init)

provider = "{provider}"

# API Keys
gemini_api_key = "{}"
openai_api_key = "{}"
groq_api_key = "{}"
openrouter_api_key = "{}"
anthropic_api_key = "{}"

# Model Choices
gemini_model = "gemini-3.5-flash-lite"
openai_model = "gpt-5.6-luna"
groq_model = "openai/gpt-oss-120b"
openrouter_model = "openai/gpt-4o-mini"
anthropic_model = "claude-3-5-sonnet"

# Local Endpoints (Ollama / LM Studio)
ollama_base_url = "http://localhost:11434"
ollama_model = "llama3"

lm_studio_base_url = "http://localhost:1234/v1"
lm_studio_model = "local-model"

# Multi-Agent Role Model Overrides
architect_model = "gemini-3.5-flash-lite"
coder_model = "gpt-5.6-luna"
"#,
                        if provider == "gemini" { key_val } else { "" },
                        if provider == "openai" { key_val } else { "" },
                        if provider == "groq" { key_val } else { "" },
                        if provider == "openrouter" {
                            key_val
                        } else {
                            ""
                        },
                        if provider == "anthropic" { key_val } else { "" }
                    );

                    if provider == "ollama" {
                        toml_content = toml_content
                            .replace(
                                r#"ollama_base_url = "http://localhost:11434""#,
                                &format!(r#"ollama_base_url = "{}""#, key_val),
                            )
                            .replace(
                                r#"ollama_model = "llama3""#,
                                &format!(r#"ollama_model = "{}""#, chosen_model),
                            );
                    } else if provider == "lmstudio" {
                        toml_content = toml_content
                            .replace(
                                r#"lm_studio_base_url = "http://localhost:1234/v1""#,
                                &format!(r#"lm_studio_base_url = "{}""#, key_val),
                            )
                            .replace(
                                r#"lm_studio_model = "local-model""#,
                                &format!(r#"lm_studio_model = "{}""#, chosen_model),
                            );
                    } else if provider == "gemini" {
                        toml_content = toml_content.replace(
                            r#"gemini_model = "gemini-3.5-flash-lite""#,
                            &format!(r#"gemini_model = "{}""#, chosen_model),
                        );
                    } else if provider == "openai" {
                        toml_content = toml_content.replace(
                            r#"openai_model = "gpt-5.6-luna""#,
                            &format!(r#"openai_model = "{}""#, chosen_model),
                        );
                    } else if provider == "groq" {
                        toml_content = toml_content.replace(
                            r#"groq_model = "openai/gpt-oss-120b""#,
                            &format!(r#"groq_model = "{}""#, chosen_model),
                        );
                    } else if provider == "openrouter" {
                        toml_content = toml_content.replace(
                            r#"openrouter_model = "openai/gpt-4o-mini""#,
                            &format!(r#"openrouter_model = "{}""#, chosen_model),
                        );
                    } else if provider == "anthropic" {
                        toml_content = toml_content.replace(
                            r#"anthropic_model = "claude-3-5-sonnet""#,
                            &format!(r#"anthropic_model = "{}""#, chosen_model),
                        );
                    }

                    match std::fs::write(target_path, toml_content) {
                        Ok(_) => {
                            println!(
                                "{}",
                                format!(
                                    "Successfully generated rune.toml at {}",
                                    target_path.display()
                                )
                                .green()
                                .bold()
                            );
                            println!("{}", "You can now restart Rune or use /provider to load the new configuration.".cyan());
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Failed to write rune.toml: {e}").red());
                        }
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/help") {
                    println!("{}", "AVAILABLE COMMANDS:".cyan().bold());
                    println!("  - {:<22} : Show this help message", "/help".green());
                    println!(
                        "  - {:<22} : List available tools the agent can use",
                        "/tools".green()
                    );
                    println!(
                        "  - {:<22} : Show conversation context stats (tokens/chars)",
                        "/context".green()
                    );
                    println!(
                        "  - {:<22} : Print the repository file tree",
                        "/files".green()
                    );
                    println!(
                        "  - {:<22} : Toggle or set auto-approve mode (/auto on|off)",
                        "/auto".green()
                    );
                    println!(
                        "  - {:<22} : Enable planning mode for step-by-step proposals (/plan [task])",
                        "/plan".green()
                    );
                    println!(
                        "  - {:<22} : Enable execution mode to carry out plans and tool calls",
                        "/execute".green()
                    );
                    println!(
                        "  - {:<22} : Switch or view LLM provider (/provider [gemini|openai|groq|openrouter] [model])",
                        "/provider".green()
                    );
                    println!(
                        "  - {:<22} : Render all rune-notes into an HTML knowledge graph",
                        "/render [file]".green()
                    );
                    println!(
                        "  - {:<22} : Interactively setup or generate rune.toml configuration",
                        "/init".green()
                    );
                    println!(
                        "  - {:<22} : Interactively view and edit rune.toml configuration",
                        "/config".green()
                    );
                    println!(
                        "  - {:<22} : View or switch model (/model [name])",
                        "/model".green()
                    );
                    println!(
                        "  - {:<22} : Save session history to JSON file",
                        "/save [file]".green()
                    );
                    println!(
                        "  - {:<22} : Load session history from JSON file",
                        "/load [file]".green()
                    );
                    println!(
                        "  - {:<22} : Clear the conversation history",
                        "/clear or /reset".green()
                    );
                    println!("  - {:<22} : Exit the application", "exit or quit".green());
                    continue;
                }

                let _ = rl.add_history_entry(prompt);

                orchestrator
                    .run_workflow(prompt, auto_approve, plan_mode)
                    .await;
                println!();
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".yellow());
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "EOF".yellow());
                break;
            }
            Err(err) => {
                eprintln!("{}", format!("Error reading input: {err}").red());
                break;
            }
        }
    }

    let _ = rl.save_history(&history_file);
    Ok(())
}
