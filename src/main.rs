use std::env;
use std::error::Error;
use rune::api::Agent;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let workspace_root = env::current_dir().expect("Failed to get current directory");

    let api_key = env::var("GEMINI_API_KEY")
        .expect("Please set the GEMINI_API_KEY environment variable");

    let model = env::var("GEMINI_MODEL")
        .unwrap_or("gemini-3.5-flash-lite".to_string());

    let mut agent = Agent::new(api_key, workspace_root, model);

    let mut rl = DefaultEditor::new()?;
    let history_file = env::temp_dir().join(".rune_history");
    let _ = rl.load_history(&history_file);

    println!("=== Rune Coding Harness ===");
    println!("Type your prompt, or use /help for available commands.");
    println!("Type 'exit' or 'quit' to end session.\n");

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let prompt = line.trim();

                if prompt.is_empty() {
                    continue;
                }

                if prompt.eq_ignore_ascii_case("exit") || prompt.eq_ignore_ascii_case("quit") {
                    println!("exiting.");
                    break;
                }

                if prompt.eq_ignore_ascii_case("/clear") || prompt.eq_ignore_ascii_case("/reset") {
                    agent.clear_history();
                    println!("Cleared conversation history.");
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/tools") {
                    println!("AVAILABLE TOOLS:");
                    println!("  - list_files       : List files and directories in a given path");
                    println!("  - read_file        : Read the full text content of a file");
                    println!("  - write_file       : Write or overwrite content to a file (requires confirmation)");
                    println!("  - execute_commands : Run shell commands like cargo check (requires confirmation)");
                    continue;
                }

                if prompt.eq_ignore_ascii_case("/help") {
                    println!("AVAILABLE COMMANDS:");
                    println!("  /help              : Show this help message");
                    println!("  /tools             : List available tools the agent can use");
                    println!("  /clear or /reset   : Clear the conversation history");
                    println!("  exit or quit       : Exit the application");
                    continue;
                }

                let _ = rl.add_history_entry(prompt);

                agent.run(prompt).await;
                println!();
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("EOF");
                break;
            }
            Err(err) => {
                eprintln!("Error reading input: {err}");
                break;
            }
        }
    }

    let _ = rl.save_history(&history_file);
    Ok(())
}
