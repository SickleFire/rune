use crate::tools::ToolExecutor;
use serde::{Deserialize, Serialize};
use std::eprintln;
use std::path::Path;
use std::path::PathBuf;
use std::println;
use futures_util::StreamExt;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::task;

#[derive(Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Serialize, Clone)]
pub struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GeminiCandidate {
    pub content: GeminiResponseContent,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GeminiResponseContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Serialize, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct Agent {
    client: reqwest::Client,
    api_key: String,
    model: String,
    history: Vec<GeminiContent>,
    executor: ToolExecutor,
    tools: Vec<Tool>,
    workspace_root: PathBuf,
}

impl Agent {
    pub fn new(api_key: String, workspace_root: PathBuf, model: String) -> Self {
        let mut agent = Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            history: Vec::new(),
            executor: ToolExecutor::new(workspace_root.clone()),
            tools: get_tool_declarations(),
            workspace_root,
        };
        agent.initialize_system_context();
        agent
    }

    fn initialize_system_context(&mut self) {
        let project_map = self.build_project_map(&self.workspace_root, "");
        let system_prompt = format!(
            "You are Rune, an expert AI coding assistant integrated into a software development workspace.\n\
            \n\
            ## Repository Architecture & File Tree:\n\
            {}\n\
            \n\
            ## Guidelines & Smart Context:\n\
            - You have global awareness of the repository architecture from the file tree above.\n\
            - When the user mentions specific files using `@filename` (e.g. `@src/api.rs`), those files are automatically loaded and injected into your prompt context.\n\
            - Use `search_code` (powered by the `cix` indexed search engine) to instantly search for functions, symbols, or patterns across the repository when you need to locate code.\n\
            - Use `read_file`, `list_files`, `write_file`, and `execute_commands` as needed to inspect and modify code.\n\
            - Be concise, precise, and proactive.",
            project_map
        );

        self.history.push(GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: Some(format!("[SYSTEM ARCHITECTURE INITIALIZATION]\n{}", system_prompt)),
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });
        self.history.push(GeminiContent {
            role: Some("model".to_string()),
            parts: vec![GeminiPart {
                text: Some("Understood. I have loaded the repository architecture and am ready to assist you. You can `@mention` files or ask me to search and modify code.".to_string()),
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });
    }

    fn build_project_map(&self, dir: &Path, prefix: &str) -> String {
        let mut output = String::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return format!("{} [Error reading directory]\n", prefix),
        };

        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        paths.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        let filtered: Vec<_> = paths
            .into_iter()
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str != "target" && name_str != ".git" && name_str != ".DS_Store" && !name_str.ends_with(".rs.bk")
            })
            .collect();

        let count = filtered.len();
        for (i, entry) in filtered.into_iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            let file_path = entry.path();

            if file_path.is_dir() {
                output.push_str(&format!("{}{}{}/\n", prefix, connector, name_str));
                let extension = if is_last { "    " } else { "│   " };
                let new_prefix = format!("{}{}", prefix, extension);
                output.push_str(&self.build_project_map(&file_path, &new_prefix));
            } else {
                output.push_str(&format!("{}{}{}\n", prefix, connector, name_str));
            }
        }

        output
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.initialize_system_context();
    }

    async fn execute_tool(&self, call: &FunctionCall) -> String {
        if self.is_destructive_tool(call) {
            if !confirm_execution(call).await {
                println!("\n[Execution cancelled by user for: {}]", call.name);
                return "Tool execution rejected by user.".to_string();
            }
        }

        match call.name.as_str() {
            "list_files" => {
                let path = call.args["path"].as_str().unwrap_or(".");
                match self.executor.list_files(Path::new(path)).await {
                    Ok(files) => files,
                    Err(e) => format!("Error listing files: {e}"),
                }
            }
            "read_file" => {
                let path = call.args["path"].as_str().unwrap_or("");
                match self.executor.read_file(Path::new(path)).await {
                    Ok(content) => content,
                    Err(e) => format!("Error reading file: {e}"),
                }
            }
            "write_file" => {
                let path = call.args["path"].as_str().unwrap_or("");
                let content = call.args["content"].as_str().unwrap_or("");
                match self.executor.write_file(Path::new(path), content).await {
                    Ok(_) => "File Written Successfully.".to_string(),
                    Err(e) => format!("Error writing file: {e}"),
                }
            }
            "search_code" => {
                let query = call.args["query"].as_str().unwrap_or("");
                match self.executor.search_code(query).await {
                    Ok(output) => output,
                    Err(e) => format!("Error executing cix search: {e}"),
                }
            }
            "execute_commands" => {
                let cmd = call.args["cmd"].as_str().unwrap_or("");
                match self.executor.execute_commands(cmd).await {
                    Ok(output) => output,
                    Err(e) => format!("Error executing command: {e}"),
                }
            }
            unknown => format!("Error: Unknown tool function '{unknown}'"),
        }
    }

    fn is_destructive_tool(&self, call: &FunctionCall) -> bool {
        let is_destructive = matches!(call.name.as_str(), "write_file" | "execute_commands");
        is_destructive
    }

    pub async fn run(&mut self, raw_prompt: &str) {
        let processed_prompt = self.resolve_mentions(raw_prompt).await;

        self.history.push(GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: Some(processed_prompt),
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model,
            self.api_key
        );

        loop {
            let request = GeminiRequest {
                contents: self.history.clone(),
                tools: Some(self.tools.clone()),
            };

            let response = match self.client.post(&url).json(&request).send().await {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("HTTP Request failed: {e}");
                    break;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let err_text = response.text().await.unwrap_or_default();
                eprintln!("API Error ({status}): {err_text}");
                break;
            }

            let mut stream = response.bytes_stream();
            let mut accumulated_text = String::new();
            let mut tool_calls: Vec<(FunctionCall, Option<String>)> = Vec::new();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("\n[Stream error: {e}]");
                        break;
                    }
                };

                let text = String::from_utf8_lossy(&bytes);
                buffer.push_str(&text);

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);

                    if line.starts_with("data: ") {
                        let json_str = &line["data: ".len()..];

                        if let Ok(parsed) = serde_json::from_str::<GeminiResponse>(json_str) {
                            if let Some(candidate) = parsed.candidates.and_then(|c| c.into_iter().next()) {
                                for part in candidate.content.parts {
                                    if let Some(t) = &part.text {
                                        print!("{t}");
                                        let _ = io::stdout().flush();
                                        accumulated_text.push_str(t);
                                    }
                                    if let Some(call) = part.function_call {
                                        if !tool_calls.iter().any(|(c, _)| c.name == call.name && c.args == call.args) {
                                            tool_calls.push((call, part.thought_signature));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut model_parts = Vec::new();
            if !accumulated_text.is_empty() {
                model_parts.push(GeminiPart {
                    text: Some(accumulated_text),
                    function_call: None,
                    function_response: None,
                    thought_signature: None,
                });
            }

            for (call, sig) in &tool_calls {
                model_parts.push(GeminiPart {
                    text: None,
                    function_call: Some(call.clone()),
                    function_response: None,
                    thought_signature: sig.clone(),
                });
            }

            if !model_parts.is_empty() {
                self.history.push(GeminiContent {
                    role: Some("model".to_string()),
                    parts: model_parts,
                });
            }

            if tool_calls.is_empty() {
                println!();
                break;
            }

            let mut response_parts = Vec::new();
            for (call, _) in tool_calls {
                println!("\n[Executing tool: {}]", call.name);
                let output = self.execute_tool(&call).await;

                response_parts.push(GeminiPart {
                    text: None,
                    function_call: None,
                    function_response: Some(FunctionResponse {
                        name: call.name,
                        response: serde_json::json!({ "result": output }),
                    }),
                    thought_signature: None,
                });
            }

            self.history.push(GeminiContent {
                role: Some("user".to_string()),
                parts: response_parts,
            });
        }
    }

    async fn resolve_mentions(&self, raw_prompt: &str) -> String {
        let words: Vec<&str> = raw_prompt.split_whitespace().collect();
        let mut resolved_prompt = raw_prompt.to_string();
        let mut injected_files = Vec::new();

        for word in words {
            if word.starts_with('@') && word.len() > 1 {
                let file_path_str = &word[1..];
                let clean_path = file_path_str.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_');
                
                if !clean_path.is_empty() {
                    match self.executor.read_file(Path::new(clean_path)).await {
                        Ok(content) => {
                            injected_files.push(format!("\n\n[Auto-injected content of @{}]:\n```\n{}\n```", clean_path, content));
                            println!("[Rune: Auto-scraped @{} into context]", clean_path);
                        }
                        Err(e) => {
                            println!("[Rune: Warning: failed to read @{}: {}]", clean_path, e);
                        }
                    }
                }
            }
        }

        if !injected_files.is_empty() {
            for injection in injected_files {
                resolved_prompt.push_str(&injection);
            }
        }

        resolved_prompt
    }
}

async fn confirm_execution(call: &FunctionCall) -> bool {
    let call = call.clone();
    
    task::spawn_blocking(move || -> bool {
        println!("\n The agent wants to execute a potentially destructive tool:");
        println!("   Function : {}", call.name);
        println!("   Arguments: {}", call.args);
        println!("   Allow execution? [y/N]: ");
        
        let _ = io::stdout().flush();
    
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim().to_lowercase();
            trimmed == "y" || trimmed == "yes"
        } else {
            false
        }
    })
    .await
    .unwrap_or(false)
}

pub fn get_tool_declarations() -> Vec<Tool> {
    let tools = vec![Tool {
        function_declarations: vec![
            FunctionDeclaration {
                name: "list_files".to_string(),
                description: "List files and directories in a given relative path.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "path": {
                            "type": "STRING",
                            "description": "Relative directory path (e.g., '.' or 'src')"
                        }
                    },
                    "required": ["path"]
                }),
            },
            FunctionDeclaration {
                name: "read_file".to_string(),
                description: "Read the full text content of a file.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "path": {
                            "type": "STRING",
                            "description": "Global or relative file path (e.g., 'src/main.rs')"
                        }
                    },
                    "required": ["path"]
                }),
            },
            FunctionDeclaration {
                name: "write_file".to_string(),
                description: "Write or overwrite content to a specified file path.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "path": {
                            "type": "STRING",
                            "description": "Relative file path"
                        },
                        "content": {
                            "type": "STRING",
                            "description": "Text content to write into the file"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            FunctionDeclaration {
                name: "search_code".to_string(),
                description: "Search the codebase using the cix indexed search engine for functions, symbols, or queries.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "query": {
                            "type": "STRING",
                            "description": "Search query term or pattern (e.g. 'ToolExecutor' or 'sanitize_path')"
                        }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "execute_commands".to_string(),
                description: "Execute a shell command.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "cmd": {
                            "type": "STRING",
                            "description": "Command string to execute (e.g., 'cargo check')"
                        }
                    },
                    "required": ["cmd"]
                }),
            },
        ],
    }];

    tools
}
