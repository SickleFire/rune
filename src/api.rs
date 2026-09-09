use crate::tools::AgentTool;
use crate::tools::ToolExecutor;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::eprintln;
use std::io::Write as StdWrite;
use std::path::Path;
use std::path::PathBuf;
use std::println;
use std::sync::Arc;
use tokio::task;

#[derive(Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Serialize, Deserialize, Clone)]
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
    pub model: String,
    history: Vec<GeminiContent>,
    executor: ToolExecutor,
    tools: Vec<Tool>,
    dynamic_tools: HashMap<String, Arc<dyn AgentTool>>,
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
            dynamic_tools: HashMap::new(),
            workspace_root,
        };

        agent.initialize_system_context();
        agent
    }

    /// Builder method to dynamically attach any tool module
    pub fn with_tool(mut self, tool: Arc<dyn AgentTool>) -> Self {
        let decl = tool.declaration();

        self.dynamic_tools.insert(decl.name.clone(), tool);

        if self.tools.is_empty() {
            self.tools.push(Tool {
                function_declarations: vec![decl],
            });
        } else {
            self.tools[0].function_declarations.push(decl);
        }

        self
    }

    pub fn get_workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub async fn undo(&self) -> Result<String, std::io::Error> {
        self.executor.undo_git_checkpoint().await
    }

    pub fn get_history_stats(&self) -> (usize, usize) {
        let message_count = self.history.len();
        let total_chars: usize = self
            .history
            .iter()
            .flat_map(|c| &c.parts)
            .filter_map(|p| p.text.as_deref())
            .map(|t| t.len())
            .sum();
        (message_count, total_chars)
    }

    pub fn save_session(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_session(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let loaded_history: Vec<GeminiContent> = serde_json::from_str(&json)?;
        self.history = loaded_history;
        Ok(())
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
            - Use `read_file`, `list_files`, `write_file`, `patch_file`, `execute_commands`, and `execute_batch` as needed to inspect and modify code.\n\
            - Prefer `patch_file` over `write_file` for surgical code edits using search and replace blocks.\n\
            - Be concise, precise, and proactive.",
            project_map
        );

        self.history.push(GeminiContent {
            role: Some("user".to_string()),
            parts: vec![GeminiPart {
                text: Some(format!(
                    "[SYSTEM ARCHITECTURE INITIALIZATION]\n{}",
                    system_prompt
                )),
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

    pub fn build_project_map(&self, dir: &Path, prefix: &str) -> String {
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
                name_str != "target"
                    && name_str != ".git"
                    && name_str != ".DS_Store"
                    && !name_str.ends_with(".rs.bk")
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

    async fn execute_tool(&self, call: &FunctionCall, auto_approve: bool) -> String {
        if let Some(dynamic_tool) = self.dynamic_tools.get(&call.name) {
            if dynamic_tool.is_destructive() {
                if !auto_approve && !confirm_execution(call).await {
                    println!("\n[Execution cancelled by user for: {}]", call.name);
                    return "Tool execution rejected by user.".to_string();
                }
            }
            return dynamic_tool.execute(call.args.clone()).await;
        }

        if call.name.as_str() == "patch_file" {
            let path = call.args["path"].as_str().unwrap_or("");
            let search = call.args["search"].as_str().unwrap_or("");
            let replace = call.args["replace"].as_str().unwrap_or("");

            match self
                .executor
                .preview_patch(Path::new(path), search, replace)
                .await
            {
                Ok((safe_path, new_content)) => {
                    let search_len = search.len();
                    let replace_len = replace.len();
                    if !auto_approve && !confirm_execution(call).await {
                        println!("\n[Execution cancelled by user for: {}]", call.name);
                        return "Tool execution rejected by user.".to_string();
                    }
                    match self
                        .executor
                        .apply_patch(&safe_path, &new_content, search_len, replace_len)
                        .await
                    {
                        Ok(_) => "File Patched Successfully.".to_string(),
                        Err(e) => format!("Error applying patch: {e}"),
                    }
                }
                Err(e) => format!("Error previewing patch: {e}"),
            }
        } else if call.name.as_str() == "write_file" {
            let path = call.args["path"].as_str().unwrap_or("");
            let content = call.args["content"].as_str().unwrap_or("");

            match self.executor.preview_write(Path::new(path), content).await {
                Ok(safe_path) => {
                    if !auto_approve && !confirm_execution(call).await {
                        println!("\n[Execution cancelled by user for: {}]", call.name);
                        return "Tool execution rejected by user.".to_string();
                    }
                    match self.executor.apply_write(&safe_path, content).await {
                        Ok(msg) => msg,
                        Err(e) => format!("Error writing file: {e}"),
                    }
                }
                Err(e) => format!("Error previewing write: {e}"),
            }
        } else {
            if self.is_destructive_tool(call) {
                if !auto_approve && !confirm_execution(call).await {
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
                "search_code" => {
                    let query = call.args["query"].as_str().unwrap_or("");
                    match self.executor.search_code(query).await {
                        Ok(output) => output,
                        Err(e) => format!("Error executing cix search: {e}"),
                    }
                }
                "git_status" => match self.executor.git_status().await {
                    Ok(output) => output,
                    Err(e) => format!("Error running git status: {e}"),
                },
                "git_diff" => {
                    let path = call.args["path"].as_str();
                    match self.executor.git_diff(path).await {
                        Ok(output) => output,
                        Err(e) => format!("Error running git diff: {e}"),
                    }
                }
                "git_commit" => {
                    let message = call.args["message"].as_str().unwrap_or("rune update");
                    match self.executor.git_commit(message).await {
                        Ok(output) => output,
                        Err(e) => format!("Error running git commit: {e}"),
                    }
                }
                "undo_git_checkpoint" => match self.executor.undo_git_checkpoint().await {
                    Ok(output) => output,
                    Err(e) => format!("Error undoing git checkpoint: {e}"),
                },
                "execute_commands" => {
                    let cmd = call.args["cmd"].as_str().unwrap_or("");
                    match self.executor.execute_commands(cmd).await {
                        Ok(output) => output,
                        Err(e) => format!("Error executing command: {e}"),
                    }
                }
                "execute_batch" => {
                    let cmds_val = &call.args["commands"];
                    let cmds: Vec<String> = if let Some(arr) = cmds_val.as_array() {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    } else if let Some(s) = cmds_val.as_str() {
                        vec![s.to_string()]
                    } else {
                        Vec::new()
                    };

                    match self.executor.execute_batch(&cmds).await {
                        Ok(output) => output,
                        Err(e) => format!("Error executing batch commands: {e}"),
                    }
                }
                unknown => format!("Error: Unknown tool function '{unknown}'"),
            }
        }
    }

    fn is_destructive_tool(&self, call: &FunctionCall) -> bool {
        if let Some(tool) = self.dynamic_tools.get(&call.name) {
            return tool.is_destructive();
        }

        let is_destructive = matches!(
            call.name.as_str(),
            "write_file"
                | "patch_file"
                | "execute_commands"
                | "execute_batch"
                | "git_commit"
                | "undo_git_checkpoint"
        );
        is_destructive
    }

    fn is_read_only_tool(&self, call: &FunctionCall) -> bool {
        if let Some(tool) = self.dynamic_tools.get(&call.name) {
            return !tool.is_destructive();
        }

        matches!(
            call.name.as_str(),
            "list_files" | "read_file" | "search_code" | "git_status" | "git_diff"
        )
    }

    pub async fn run(&mut self, raw_prompt: &str, auto_approve: bool) {
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
            self.model, self.api_key
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
                            if let Some(candidate) =
                                parsed.candidates.and_then(|c| c.into_iter().next())
                            {
                                for part in candidate.content.parts {
                                    if let Some(t) = &part.text {
                                        print!("{t}");
                                        let _ = std::io::stdout().flush();
                                        accumulated_text.push_str(t);
                                    }
                                    if let Some(call) = part.function_call {
                                        if !tool_calls.iter().any(|(c, _)| {
                                            c.name == call.name && c.args == call.args
                                        }) {
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

            let mut read_only_calls = Vec::new();
            let mut mutating_calls = Vec::new();

            for (call, sig) in tool_calls {
                if self.is_read_only_tool(&call) {
                    read_only_calls.push((call, sig));
                } else {
                    mutating_calls.push((call, sig));
                }
            }

            if !read_only_calls.is_empty() {
                let executor = &self.executor;
                let ro_dynamic: HashMap<String, Arc<dyn AgentTool>> = read_only_calls
                    .iter()
                    .filter_map(|(call, _)| {
                        self.dynamic_tools
                            .get(&call.name)
                            .map(|t| (call.name.clone(), Arc::clone(t)))
                    })
                    .collect();

                let futures = read_only_calls.into_iter().map(|(call, _sig)| {
                    let ro_dynamic = ro_dynamic.clone();
                    async move {
                        println!("\n[Executing read-only tool concurrently: {}]", call.name);

                        if let Some(tool) = ro_dynamic.get(&call.name) {
                            let output = tool.execute(call.args.clone()).await;
                            return (call.name, output);
                        }

                        let output = match call.name.as_str() {
                            "list_files" => {
                                let path = call.args["path"].as_str().unwrap_or(".");
                                match executor.list_files(Path::new(path)).await {
                                    Ok(files) => files,
                                    Err(e) => format!("Error listing files: {e}"),
                                }
                            }
                            "read_file" => {
                                let path = call.args["path"].as_str().unwrap_or("");
                                match executor.read_file(Path::new(path)).await {
                                    Ok(content) => content,
                                    Err(e) => format!("Error reading file: {e}"),
                                }
                            }
                            "search_code" => {
                                let query = call.args["query"].as_str().unwrap_or("");
                                match executor.search_code(query).await {
                                    Ok(output) => output,
                                    Err(e) => format!("Error executing cix search: {e}"),
                                }
                            }
                            "git_status" => match executor.git_status().await {
                                Ok(output) => output,
                                Err(e) => format!("Error running git status: {e}"),
                            },
                            "git_diff" => {
                                let path = call.args["path"].as_str();
                                match executor.git_diff(path).await {
                                    Ok(output) => output,
                                    Err(e) => format!("Error running git diff: {e}"),
                                }
                            }
                            _ => format!("Error: Unknown read-only tool '{}'", call.name),
                        };
                        (call.name, output)
                    }
                });

                let results = futures_util::future::join_all(futures).await;
                for (name, output) in results {
                    response_parts.push(GeminiPart {
                        text: None,
                        function_call: None,
                        function_response: Some(FunctionResponse {
                            name,
                            response: serde_json::json!({ "result": output }),
                        }),
                        thought_signature: None,
                    });
                }
            }

            for (call, _) in mutating_calls {
                println!("\n[Executing tool sequentially: {}]", call.name);
                let output = self.execute_tool(&call, auto_approve).await;

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
                let clean_path = file_path_str.trim_matches(|c: char| {
                    !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_'
                });

                if !clean_path.is_empty() {
                    match self.executor.read_file(Path::new(clean_path)).await {
                        Ok(content) => {
                            injected_files.push(format!(
                                "\n\n[Auto-injected content of @{}]:\n```\n{}\n```",
                                clean_path, content
                            ));
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
        println!("\n The agent wants to execute a potentially destructive tool execution:");
        println!("   Function : {}", call.name);
        if call.name == "execute_commands" || call.name == "execute_batch" {
            println!("   Args     : {}", call.args);
        }
        println!("   Allow execution? [y/N]: ");

        let _ = std::io::stdout().flush();

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
                            "description": "Relative file path"
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
                description: "Write or overwrite content to a specified file path (use for creating new files or complete overwrites).".to_string(),
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
                name: "patch_file".to_string(),
                description: "Perform a surgical search-and-replace patch on a file. Preferred for editing existing files to avoid rewriting large files and reduce token usage.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "path": {
                            "type": "STRING",
                            "description": "Relative file path"
                        },
                        "search": {
                            "type": "STRING",
                            "description": "Exact search block snippet to locate in the file"
                        },
                        "replace": {
                            "type": "STRING",
                            "description": "Replacement block snippet"
                        }
                    },
                    "required": ["path", "search", "replace"]
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
                            "description": "Search query term or print pattern (e.g. 'ToolExecutor' or 'sanitize_path')"
                        }
                    },
                    "required": ["query"]
                }),
            },
            FunctionDeclaration {
                name: "git_status".to_string(),
                description: "Check the current git status of the repository.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {},
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "git_diff".to_string(),
                description: "Show changes between the working tree and index or a commit, optionally for a specific file path.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "path": {
                            "type": "STRING",
                            "description": "Optional file path to diff"
                        }
                    },
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "git_commit".to_string(),
                description: "Stage all changes and create a git commit with a message.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "message": {
                            "type": "STRING",
                            "description": "Commit message"
                        }
                    },
                    "required": ["message"]
                }),
            },
            FunctionDeclaration {
                name: "undo_git_checkpoint".to_string(),
                description: "Undo the last file mutation made by the agent by popping the git stash checkpoint.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {},
                    "required": []
                }),
            },
            FunctionDeclaration {
                name: "execute_commands".to_string(),
                description: "Execute a single shell command.".to_string(),
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
            FunctionDeclaration {
                name: "execute_batch".to_string(),
                description: "Execute a batch of sequential shell commands in the workspace.".to_string(),
                parameters: serde_json::json!({
                    "type": "OBJECT",
                    "properties": {
                        "commands": {
                            "type": "ARRAY",
                            "items": {
                                "type": "STRING"
                            },
                            "description": "List of shell command strings to execute sequentially (e.g. ['cargo check', 'cargo test'])"
                        }
                    },
                    "required": ["commands"]
                }),
            },
        ],
    }];

    tools
}
