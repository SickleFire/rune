use std::env;
use std::error::Error;
use rune::api::Agent;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let workspace_root = env::current_dir().expect("Failed to get current directory");

    let api_key = env::var("GEMINI_API_KEY")
        .expect("Please set the GEMINI_API_KEY environment variable");

    let model = env::var("GEMINI_MODEL")
        .unwrap_or("gemini-3.5-flash-lite".to_string());

    let mut agent = Agent::new(api_key, workspace_root, model);

    let mut auto_approve = false;
    let mut rl = DefaultEditor::new()?;
    let history_file = env::temp_dir().join(".rune_history");
    let _ = rl.load_history(&history_file);

    println!("{}", "=== Rune Coding Harness ===".cyan().bold());
    println!("{}", "Type your prompt, or use /help for available commands.".bright_black());
    println!("{}", "Type 'exit' or 'quit' to end session.\n".bright_black());

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

                if prompt.eq_ignore_ascii_case("/context") || prompt.eq_ignore_ascii_case("/tokens") {
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
                        println!("Auto-approve mode is now: {}", if auto_approve { "ENABLED".green() } else { "DISABLED".yellow() });
                    }
                    continue;
                }

                if prompt.starts_with("/model") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    if parts.len() > 1 {
                        agent.model = parts[1].to_string();
                        println!("{}", format!("Switched active model to: {}", agent.model).green());
                    } else {
                        println!("{}", format!("Current active model: {}", agent.model).cyan());
                        println!("Usage: /model <model_name>");
                    }
                    continue;
                }

                if prompt.starts_with("/save") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    let filename = if parts.len() > 1 { parts[1] } else { "rune_session.json" };
                    match agent.save_session(std::path::Path::new(filename)) {
                        Ok(_) => println!("{}", format!("Successfully saved session history to '{}'", filename).green()),
                        Err(e) => eprintln!("{}", format!("Failed to save session: {e}").red()),
                    }
                    continue;
                }

                if prompt.starts_with("/load") {
                    let parts: Vec<&str> = prompt.split_whitespace().collect();
                    let filename = if parts.len() > 1 { parts[1] } else { "rune_session.json" };
                    match agent.load_session(std::path::Path::new(filename)) {
                        Ok(_) => println!("{}", format!("Successfully loaded session history from '{}'", filename).green()),
                        Err(e) => eprintln!("{}", format!("Failed to load session: {e}").red()),
                    }
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/tools") {
                    println!("{}", "AVAILABLE TOOLS:".cyan().bold());
                    println!("  - {:<18} : List files and directories in a given path", "list_files".green());
                    println!("  - {:<18} : Read the full text content of a file", "read_file".green());
                    println!("  - {:<18} : Write or overwrite content to a file (requires confirmation)", "write_file".green());
                    println!("  - {:<18} : Surgical search-and-replace patch on a file (requires confirmation)", "patch_file".green());
                    println!("  - {:<18} : Instant BM25 code search via cix binary", "search_code".green());
                    println!("  - {:<18} : Run a single shell command (requires confirmation)", "execute_commands".green());
                    println!("  - {:<18} : Run a batch of sequential shell commands (requires confirmation)", "execute_batch".green());
                    println!("  - {:<18} : Check the current git status of the repository", "git_status".green());
                    println!("  - {:<18} : Show changes in working tree or file path", "git_diff".green());
                    println!("  - {:<18} : Stage all changes and create a git commit (requires confirmation)", "git_commit".green());
                    println!("  - {:<18} : Undo the last file mutation via git stash checkpoint", "undo_git_checkpoint".green());
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/help") {
                    println!("{}", "AVAILABLE COMMANDS:".cyan().bold());
                    println!("  - {:<22} : Show this help message", "/help".green());
                    println!("  - {:<22} : List available tools the agent can use", "/tools".green());
                    println!("  - {:<22} : Show conversation context stats (tokens/chars)", "/context".green());
                    println!("  - {:<22} : Print the repository file tree", "/files".green());
                    println!("  - {:<22} : Toggle or set auto-approve mode (/auto on|off)", "/auto".green());
                    println!("  - {:<22} : View or switch Gemini model (/model [name])", "/model".green());
                    println!("  - {:<22} : Save session history to JSON file", "/save [file]".green());
                    println!("  - {:<22} : Load session history from JSON file", "/load [file]".green());
                    println!("  - {:<22} : Clear the conversation history", "/clear or /reset".green());
                    println!("  - {:<22} : Exit the application", "exit or quit".green());
                    continue;
                }

                let _ = rl.add_history_entry(prompt);

                agent.run(prompt, auto_approve).await;
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
