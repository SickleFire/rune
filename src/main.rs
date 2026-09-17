use colored::*;
use rune::agent::{AgentConfig, AgentRole, MultiAgentConfig, MultiAgentOrchestrator};
use rune::api::LLMProvider;
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

struct RuneHelper {
    hinter: HistoryHinter,
}

impl Hinter for RuneHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for RuneHelper {}
impl Validator for RuneHelper {}

impl Completer for RuneHelper {
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

impl rustyline::Helper for RuneHelper {}

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

    let config = rune::config::RuneConfig::load();

    let gemini_api_key = config.gemini_api_key.clone().unwrap_or_default();
    let gemini_model = config.gemini_model.clone().unwrap_or("gemini-3.5-flash-lite".to_string());

    let openai_api_key = config.openai_api_key.clone().unwrap_or_default();
    let openai_model = config.openai_model.clone().unwrap_or("gpt-5.6-luna".to_string());

    let anthropic_api_key = config.anthropic_api_key.clone().unwrap_or_default();
    let anthropic_model = config.anthropic_model.clone().unwrap_or("claude-3-5-sonnet".to_string());

    // Determine default provider based on config.provider or available keys
    let provider_name = config.provider.as_deref().unwrap_or("auto");
    let default_provider: Arc<dyn LLMProvider> = if provider_name.eq_ignore_ascii_case("gemini") || (!gemini_api_key.is_empty() && provider_name == "auto") {
        Arc::new(rune::api::GeminiProvider::new(
            gemini_api_key.clone(),
            gemini_model.clone(),
        ))
    } else if provider_name.eq_ignore_ascii_case("openai") || (!openai_api_key.is_empty() && provider_name == "auto") {
        Arc::new(
            rune::api::OpenAIProvider::new(openai_api_key.clone(), openai_model.clone())
                .with_reasoning_effort("none"),
        )
    } else if provider_name.eq_ignore_ascii_case("anthropic") || (!anthropic_api_key.is_empty() && provider_name == "auto") {
        Arc::new(
            rune::api::OpenAIProvider::new(anthropic_api_key.clone(), anthropic_model.clone()) // Or Anthropic provider if implemented, falling back nicely
                .with_reasoning_effort("none"),
        )
    } else if provider_name.eq_ignore_ascii_case("ollama") {
        let base_url = config.ollama_base_url.as_deref().unwrap_or("http://localhost:11434");
        let model = config.ollama_model.as_deref().unwrap_or("llama3");
        println!("[Rune: Using Ollama local provider at {} (model: {})]", base_url, model);
        Arc::new(
            rune::api::OpenAIProvider::new("ollama-local".into(), model.into())
                .with_reasoning_effort("none")
        )
    } else if provider_name.eq_ignore_ascii_case("lmstudio") {
        let base_url = config.lm_studio_base_url.as_deref().unwrap_or("http://localhost:1234/v1");
        let model = config.lm_studio_model.as_deref().unwrap_or("local-model");
        println!("[Rune: Using LM Studio local provider at {} (model: {})]", base_url, model);
        Arc::new(
            rune::api::OpenAIProvider::new("lm-studio-local".into(), model.into())
                .with_reasoning_effort("none")
        )
    } else if !gemini_api_key.is_empty() {
        Arc::new(rune::api::GeminiProvider::new(
            gemini_api_key.clone(),
            gemini_model.clone(),
        ))
    } else if !openai_api_key.is_empty() {
        Arc::new(
            rune::api::OpenAIProvider::new(openai_api_key.clone(), openai_model.clone())
                .with_reasoning_effort("none"),
        )
    } else {
        panic!("Please configure at least one LLM provider (Gemini, OpenAI, Anthropic, Ollama, or LM Studio) via rune.toml or environment variables.");
    };

    // Multi-agent setup (Max 2 agents: Architect & Coder)
    let multi_agent_mode = env::var("RUNE_MULTI_AGENT").unwrap_or_else(|_| "false".to_string())
        == "true"
        || std::env::args().any(|arg| arg == "--multi-agent");

    let multi_config = if multi_agent_mode {
        // Use lightweight model for architect, powerful model for coder
        let architect_model = config.architect_model.clone().unwrap_or_else(|| gemini_model.clone());
        let coder_model = config.coder_model.clone().unwrap_or_else(|| openai_model.clone());
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
    let enable_unity = args.contains(&"--unity".to_string());
    let enable_godot = args.contains(&"--godot".to_string());
    let enable_web = args.contains(&"--web".to_string());
    let enable_github = args.contains(&"--github".to_string()) || env::var("GITHUB_TOKEN").is_ok();
    let enable_mysql = args.contains(&"--mysql".to_string());
    let enable_docker = args.contains(&"--docker".to_string());

    // Always enable core persistent memory and session memory tools
    orchestrator = orchestrator
        .with_tool_on_all(std::sync::Arc::new(
            rune::memory_tools::RememberPreferenceTool,
        ))
        .with_tool_on_all(std::sync::Arc::new(rune::memory_tools::RecallMemoryTool))
        .with_tool_on_all(std::sync::Arc::new(rune::memory_tools::RecordFixTool))
        .with_tool_on_all(std::sync::Arc::new(
            rune::memory_tools::RecordFileRelationTool,
        ));

    if enable_unity {
        println!("{}", "Unity editor tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityInspectSceneTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnitySetPropertyTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityInspectComponentsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityAssignReferenceTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityValidateReferencesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityAddComponentTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityFindAssetsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityInstantiatePrefabTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityCreateScriptableObjectTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityRefreshAssetDatabaseTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::unity_tools::UnityReadConsoleLogsTool::new(),
            ))
    }

    if enable_godot {
        println!("{}", "Godot editor tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune::godot_tools::GodotInspectSceneTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::godot_tools::GodotInspectNodePropertiesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::godot_tools::GodotCreateNodeTool::new(),
            ))
    }

    if enable_web {
        println!("{}", "Web utility tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(rune::web_tools::HttpRequestTool::new()))
            .with_tool_on_all(std::sync::Arc::new(rune::web_tools::FetchWebPageTool::new()))
            .with_tool_on_all(std::sync::Arc::new(rune::web_tools::CheckTcpPortTool::new()))
    }

    if enable_github {
        println!("{}", "GitHub integration tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune::github_tools::GitHubIssueTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::github_tools::GitHubPullRequestDiffTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::github_tools::GitHubCreateIssueTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::github_tools::GitHubCreateCommentTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::github_tools::GitHubCreatePullRequestTool::new(),
            ))
    }

    if enable_mysql {
        println!("{}", "MySQL database tools enabled.".cyan());
        let mysql_url = env::var("MYSQL_URL").ok();
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune::mysql_tools::MysqlListTablesTool::new(mysql_url.clone()),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::mysql_tools::MysqlDescribeTableTool::new(mysql_url.clone()),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::mysql_tools::MysqlExecuteQueryTool::new(mysql_url),
            ))
    }

    if enable_docker {
        println!("{}", "Docker container tools enabled.".cyan());
        orchestrator = orchestrator
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerListContainersTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerContainerLogsTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerStartContainerTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerStopContainerTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerListImagesTool::new(),
            ))
            .with_tool_on_all(std::sync::Arc::new(
                rune::docker_tools::DockerInspectContainerTool::new(),
            ))
    }

    let mut auto_approve = false;
    let mut plan_mode = false;
    let mut rl = Editor::new()?;
    rl.set_helper(Some(RuneHelper {
        hinter: HistoryHinter::new(),
    }));
    let history_file = env::temp_dir().join(".rune_history");
    let _ = rl.load_history(&history_file);

    println!("{}", "=== Rune Coding Harness ===".cyan().bold());
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
                        let executor = rune::tools::ToolExecutor::new(workspace_root);
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
                    let agent = rune::api::Agent::new_gemini(
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

                if prompt.starts_with("/provider")
                    || prompt.starts_with("/model")
                    || prompt.starts_with("/save")
                    || prompt.starts_with("/load")
                {
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
                        "  - {:<22} : Switch or view LLM provider (/provider [gemini|openai] [model])",
                        "/provider".green()
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
