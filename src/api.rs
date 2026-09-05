use crate::tools::ToolExecutor;
use serde::{Deserialize, Serialize};
use std::eprintln;
use std::path::Path;
use std::path::PathBuf;
use tokio::io;
use tokio::io::AsyncWriteExt;
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
    history: Vec<GeminiContent>,
    executor: ToolExecutor,
    tools: Vec<Tool>,
}

impl Agent {
    pub fn new(api_key: String, workspace_root: PathBuf) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            history: Vec::new(),
            executor: ToolExecutor::new(workspace_root),
            tools: get_tool_declarations(),
        }
    }

    async fn execute_tool(&self, call: &FunctionCall) -> String {
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

    pub async fn run(&mut self, prompt: &str) {
        self.history.push(GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: Some(prompt.to_string()),
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent?key={}",
            self.api_key
        );

        loop {
            let request = GeminiRequest{
                contents: self.history.clone(),
                tools: Some(self.tools.clone())
            };
            
            let response = match self.client.post(&url).json(&request).send().await {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("HTTP Request failed: {e}");
                    break;
                }
            };

            let status = response.status();

            let body_text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to read response body: {e}");
                    break;
                }
            };

            if !status.is_success() {
                eprintln!("Gemini API error ({status}): {body_text}");
                break;
            }

            let gemini_response: GeminiResponse = match serde_json::from_str(&body_text) {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("Failed to parse Gemini response: {e}\nRaw body: {body_text}");
                    break;
                }
            };

            // Extract candidate content
            let candidate_content = match gemini_response
                .candidates
                .and_then(|c| c.into_iter().next())
            {
                Some(candidate) => candidate.content,
                None => {
                    eprintln!("No candidates returned from Gemini.");
                    break;
                }
            };

            // Record model's turn in history
            self.history.push(GeminiContent {
                role: Some("model".to_string()),
                parts: candidate_content.parts.clone(),
            });

            let mut tool_calls = Vec::new();

            for part in &candidate_content.parts {
                if let Some(text) = &part.text {
                    print!("{text}");
                    let _ = io::stdout().flush();
                }
                if let Some(call) = &part.function_call {
                    tool_calls.push(call.clone());
                }
            }

            // If no tools were called, the response is finished
            if tool_calls.is_empty() {
                println!();
                break;
            }

            // Execute function calls and return function response turn
            let mut response_parts = Vec::new();
            for call in tool_calls {
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

            // Push function response back as 'user' role turn per Gemini API specs
            self.history.push(GeminiContent {
                role: Some("user".to_string()),
                parts: response_parts,
            });
        }

    }
}

pub fn get_tool_declarations() -> Vec<Tool> {
    vec![Tool {
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
                            "description": "Relative file path (e.g., 'src/main.rs')"
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
                name: "execute_commands".to_string(),
                description: "Execute a shell command on Windows CLI.".to_string(),
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
    }]
}
