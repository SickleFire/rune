use colored::*;
use rune::api::Agent;
use rustyline::Context;
use rustyline::Editor;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use std::env;
use std::error::Error;

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

    let gemini_api_key = env::var("GEMINI_API_KEY").unwrap_or_default();
    let gemini_model = env::var("GEMINI_MODEL").unwrap_or("gemini-3.5-flash-lite".to_string());

    let openai_api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
    let openai_model = env::var("OPENAI_MODEL").unwrap_or("gpt-5.6-luna".to_string());

    let mut agent = if !gemini_api_key.is_empty() {
        Agent::new_gemini(
            gemini_api_key.clone(),
            workspace_root.clone(),
            gemini_model.clone(),
        )
    } else if !openai_api_key.is_empty() {
        Agent::new_openai(
            openai_api_key.clone(),
            workspace_root.clone(),
            openai_model.clone(),
        )
    } else {
        panic!("Please set either GEMINI_API_KEY or OPENAI_API_KEY environment variable.");
    };

    let args: Vec<String> = env::args().collect();
    let enable_unity = args.contains(&"--unity".to_string());
    let enable_godot = args.contains(&"--godot".to_string());
    let enable_web = args.contains(&"--web".to_string());
    let enable_github = args.contains(&"--github".to_string()) || env::var("GITHUB_TOKEN").is_ok();
    let enable_mysql = args.contains(&"--mysql".to_string());

    if enable_unity {
        println!("{}", "Unity editor tools enabled.".cyan());
        agent = agent
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityInspectSceneTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnitySetPropertyTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityInspectComponentsTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityAssignReferenceTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityValidateReferencesTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityAddComponentTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityFindAssetsTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityInstantiatePrefabTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityRefreshAssetDatabaseTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::unity_tools::UnityReadConsoleLogsTool::new(),
            ))
    }

    if enable_godot {
        println!("{}", "Godot editor tools enabled.".cyan());
        agent = agent
            .with_tool(std::sync::Arc::new(
                rune::godot_tools::GodotInspectSceneTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::godot_tools::GodotInspectNodePropertiesTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::godot_tools::GodotCreateNodeTool::new(),
            ))
    }

    if enable_web {
        println!("{}", "Web utility tools enabled.".cyan());
        agent = agent
            .with_tool(std::sync::Arc::new(rune::web_tools::HttpRequestTool::new()))
            .with_tool(std::sync::Arc::new(rune::web_tools::FetchWebPageTool::new()))
            .with_tool(std::sync::Arc::new(rune::web_tools::CheckTcpPortTool::new()))
    }

    if enable_github {
        println!("{}", "GitHub integration tools enabled.".cyan());
        agent = agent
            .with_tool(std::sync::Arc::new(
                rune::github_tools::GitHubIssueTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::github_tools::GitHubPullRequestDiffTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::github_tools::GitHubCreateIssueTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::github_tools::GitHubCreateCommentTool::new(),
            ))
            .with_tool(std::sync::Arc::new(
                rune::github_tools::GitHubCreatePullRequestTool::new(),
            ))
    }

    if enable_mysql {
        println!("{}", "MySQL database tools enabled.".cyan());
        let mysql_url = env::var("MYSQL_URL").ok();
        agent = agent
            .with_tool(std::sync::Arc::new(
                rune::mysql_tools::MysqlListTablesTool::new(mysql_url.clone()),
            ))
            .with_tool(std::sync::Arc::new(
                rune::mysql_tools::MysqlDescribeTableTool::new(mysql_url.clone()),
            ))
            .with_tool(std::sync::Arc::new(
                rune::mysql_tools::MysqlExecuteQueryTool::new(mysql_url),
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
                    agent.clear_history();
                    println!("{}", "Cleared conversation history.".green());
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/undo") {
                    match agent.undo().await {
                        Ok(msg) => println!("{}", msg.green()),
                        Err(e) => eprintln!("{}", format!("Failed to undo: {e}").red()),
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/context") || prompt.eq_ignore_ascii_case("/tokens")
                {
                    let (msgs, chars) = agent.get_history_stats();
                    println!("{}", format!("Conversation Context: {} messages, ~{} characters (~{} estimated tokens)", msgs, chars, chars / 4).cyan());
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/files") || prompt.eq_ignore_ascii_case("/tree") {
                    let map = agent.build_project_map(agent.get_workspace_root(), "");
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
                        let plan_prompt = format!("[PLANNING MODE REQUEST] Please formulate a comprehensive, step-by-step execution plan to accomplish the following task without executing mutating tools:\n\n{instruction}");
                        agent.run_with_mode(&plan_prompt, false, true).await;
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

                if prompt.starts_with("/provider") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        let provider_name = parts[1].to_lowercase();
                        match provider_name.as_str() {
                            "gemini" => {
                                if gemini_api_key.is_empty() {
                                    eprintln!(
                                        "{}",
                                        "Error: GEMINI_API_KEY environment variable is not set."
                                            .red()
                                    );
                                } else {
                                    let model = parts
                                        .get(2)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| "gemini-3.5-flash-lite".into());
                                    let provider =
                                        std::sync::Arc::new(rune::api::GeminiProvider::new(
                                            gemini_api_key.clone(),
                                            model.clone(),
                                        ));
                                    agent.set_provider(provider, model.clone());
                                    println!(
                                        "{}",
                                        format!(
                                            "Switched active provider to: Gemini (model: {})",
                                            agent.model
                                        )
                                        .green()
                                    );
                                }
                            }
                            "openai" => {
                                if openai_api_key.is_empty() {
                                    eprintln!(
                                        "{}",
                                        "Error: OPENAI_API_KEY environment variable is not set."
                                            .red()
                                    );
                                } else {
                                    let model = parts
                                        .get(2)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| "gpt-5.6-luna".into());
                                    let provider =
                                        std::sync::Arc::new(rune::api::OpenAIProvider::new(
                                            openai_api_key.clone(),
                                            model.clone(),
                                        ));
                                    agent.set_provider(provider, model.clone());
                                    println!(
                                        "{}",
                                        format!(
                                            "Switched active provider to: OpenAI (model: {})",
                                            agent.model
                                        )
                                        .green()
                                    );
                                }
                            }
                            _ => {
                                println!("Usage: /provider [gemini|openai]");
                            }
                        }
                    } else {
                        println!(
                            "{}",
                            format!("Current active provider model: {}", agent.model).cyan()
                        );
                        println!("Usage: /provider [gemini|openai]");
                    }
                    continue;
                }

                if prompt.starts_with("/model") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        agent.model = parts[1].to_string();
                        println!(
                            "{}",
                            format!("Switched active model to: {}", agent.model).green()
                        );
                    } else {
                        println!(
                            "{}",
                            format!("Current active model: {}", agent.model).cyan()
                        );
                        println!("Usage: /model <model_name>");
                    }
                    continue;
                }

                if prompt.starts_with("/save") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    let filename = if parts.len() > 1 {
                        parts[1]
                    } else {
                        "rune_session.json"
                    };
                    match agent.save_session(std::path::Path::new(filename)) {
                        Ok(_) => println!(
                            "{}",
                            format!("Successfully saved session history to '{}'", filename).green()
                        ),
                        Err(e) => eprintln!("{}", format!("Failed to save session: {e}").red()),
                    }
                    continue;
                }

                if prompt.starts_with("/load") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    let filename = if parts.len() > 1 {
                        parts[1]
                    } else {
                        "rune_session.json"
                    };
                    match agent.load_session(std::path::Path::new(filename)) {
                        Ok(_) => println!(
                            "{}",
                            format!("Successfully loaded session history from '{}'", filename)
                                .green()
                        ),
                        Err(e) => eprintln!("{}", format!("Failed to load session: {e}").red()),
                    }
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

                agent.run_with_mode(prompt, auto_approve, plan_mode).await;
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
