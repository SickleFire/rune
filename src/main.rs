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

    let mut agent = Agent::new(api_key, workspace_root);

    let mut rl = DefaultEditor::new()?;
    let history_file = env::temp_dir().join(".rune_history");
    let _ = rl.load_history(&history_file);

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
                    println!("Goodbye!");
                    break;
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