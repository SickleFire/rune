
use std::error::Error;
use std::env;
use std::path::PathBuf;
use rune::api::Agent;

#[tokio::main]
async fn main() {
    /*let prompt = "How does mutexes work?";

    let body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: prompt.to_string(),
            }],
        }],
    };

    let client = reqwest::Client::new();

    let res = client.post(&url).json(&body).send().await.unwrap();

    let mut stream = res.bytes_stream();

    let mut buffer = String::new();
    let mut gemini_res = String::new();

    while let Some(chunk_result) = stream.next().await {
        let bytes = chunk_result.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        buffer.push_str(&text);

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer.drain(..=line_end);
            if line.starts_with("data: ") {
                let json_str = &line["data: ".len()..];

                if let Ok(parsed) = serde_json::from_str::<GeminiResponse>(json_str) {
                    println!(
                        "{:#}",
                        parsed
                            .candidates
                            .and_then(|c| c.into_iter().next())
                            .and_then(|c| c.content.parts.into_iter().next())
                            .map(|p| p.text)
                            .unwrap_or_default()
                    );

                    io::stdout().flush().unwrap();
                }
            }
        }
    }*/
    // 1. Get the current workspace root directory
    let workspace_root = env::current_dir().expect("Failed to get current directory");

    // 2. Fetch the Gemini API key from environment
    let api_key = env::var("GEMINI_API_KEY")
        .expect("Please set the GEMINI_API_KEY environment variable");

    // 3. Instantiate the agent
    let mut agent = Agent::new(api_key, workspace_root);

    // 4. Test prompt that triggers tool execution
    let prompt = "List the files in the current directory and tell me what this project is about. and any ideas what is next improvement?";
    println!("User: {prompt}\n");

    // 5. Run the loop
    agent.run(prompt).await;
}
