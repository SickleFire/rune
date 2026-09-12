use crate::tools::AgentTool;
use crate::tools::ToolExecutor;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write as StdWrite;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CanonicalMessage {
    User(String),
    Assistant {
        text: Option<String>,
        tool_calls: Vec<CanonicalToolCall>,
    },
    ToolResult(CanonicalToolResult),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CanonicalToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
}

pub struct ProviderResponse {
    pub text: String,
    pub tool_calls: Vec<CanonicalToolCall>,
}

#[derive(Serialize, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(
        &self,
        history: &[CanonicalMessage],
        tools: &[FunctionDeclaration],
        model: &str,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<ProviderResponse, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<GeminiFunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<GeminiFunctionResponse>,
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize, Clone, Debug)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize, Clone, Debug)]
struct GeminiResponseContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize, Clone)]
struct GeminiTool {
    function_declarations: Vec<FunctionDeclaration>,
}

pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    pub model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    fn to_wire(history: &[CanonicalMessage]) -> Vec<GeminiContent> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < history.len() {
            match &history[i] {
                CanonicalMessage::User(text) => {
                    out.push(GeminiContent {
                        role: Some("user".into()),
                        parts: vec![GeminiPart {
                            text: Some(text.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        }],
                    });
                    i += 1;
                }
                CanonicalMessage::Assistant { text, tool_calls } => {
                    let mut parts = Vec::new();
                    if let Some(t) = text {
                        parts.push(GeminiPart {
                            text: Some(t.clone()),
                            function_call: None,
                            function_response: None,
                            thought_signature: None,
                        });
                    }
                    for tc in tool_calls {
                        parts.push(GeminiPart {
                            text: None,
                            function_call: Some(GeminiFunctionCall {
                                name: tc.name.clone(),
                                args: tc.args.clone(),
                            }),
                            function_response: None,
                            thought_signature: tc.thought_signature.clone(),
                        });
                    }
                    out.push(GeminiContent {
                        role: Some("model".into()),
                        parts,
                    });
                    i += 1;
                }
                CanonicalMessage::ToolResult(_) => {
                    let mut parts = Vec::new();
                    while i < history.len() {
                        if let CanonicalMessage::ToolResult(r) = &history[i] {
                            parts.push(GeminiPart {
                                text: None,
                                function_call: None,
                                function_response: Some(GeminiFunctionResponse {
                                    name: r.name.clone(),
                                    response: serde_json::json!({ "result": r.content }),
                                }),
                                thought_signature: None,
                            });
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    out.push(GeminiContent {
                        role: Some("user".into()),
                        parts,
                    });
                }
            }
        }
        out
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn complete(
        &self,
        history: &[CanonicalMessage],
        tools: &[FunctionDeclaration],
        model: &str,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<ProviderResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, self.api_key
        );

        let gemini_tools: Vec<FunctionDeclaration> = tools
            .iter()
            .cloned()
            .map(|mut t| {
                gemini_normalize_schema(&mut t.parameters);
                t
            })
            .collect();

        let request = GeminiRequest {
            contents: Self::to_wire(history),
            tools: Some(vec![GeminiTool {
                function_declarations: gemini_tools,
            }]),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API error ({status}): {body}").into());
        }

        let mut stream = response.bytes_stream();
        let mut text_buf = String::new();
        let mut tool_calls: Vec<CanonicalToolCall> = Vec::new();
        let mut wire_buf = String::new();
        let mut call_counter = 0usize;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            wire_buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(end) = wire_buf.find('\n') {
                let line = wire_buf[..end].trim().to_string();
                wire_buf.drain(..=end);

                if let Some(json) = line.strip_prefix("data: ") {
                    if let Ok(resp) = serde_json::from_str::<GeminiResponse>(json) {
                        if let Some(candidate) = resp.candidates.and_then(|c| c.into_iter().next())
                        {
                            for part in candidate.content.parts {
                                if let Some(t) = &part.text {
                                    on_text(t.clone());
                                    text_buf.push_str(t);
                                }
                                if let Some(call) = part.function_call {
                                    if !tool_calls.iter().any(|tc| tc.name == call.name && tc.args == call.args) {
                                        tool_calls.push(CanonicalToolCall {
                                            id: format!("{}_{}", call.name, call_counter),
                                            name: call.name,
                                            args: call.args,
                                            thought_signature: part.thought_signature.clone(),
                                        });
                                        call_counter += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ProviderResponse {
            text: text_buf,
            tool_calls,
        })
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    stream: bool,
}

#[derive(Serialize, Clone)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCallOut>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAIToolCallOut {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunctionOut,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAIFunctionOut {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunctionSpec,
}

#[derive(Serialize)]
struct OpenAIFunctionSpec {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAIChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
}

#[derive(Deserialize, Default)]
struct OpenAIDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Deserialize)]
struct OpenAIToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<OpenAIFunctionDelta>,
}

#[derive(Deserialize)]
struct OpenAIFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    pub model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn to_wire(history: &[CanonicalMessage]) -> Vec<OpenAIMessage> {
        let mut out = Vec::new();
        for msg in history {
            match msg {
                CanonicalMessage::User(text) => {
                    out.push(OpenAIMessage {
                        role: "user".into(),
                        content: Some(text.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                    });
                }
                CanonicalMessage::Assistant { text, tool_calls } => {
                    let wire_calls: Vec<OpenAIToolCallOut> = tool_calls
                        .iter()
                        .map(|tc| OpenAIToolCallOut {
                            id: tc.id.clone(),
                            call_type: "function".into(),
                            function: OpenAIFunctionOut {
                                name: tc.name.clone(),
                                arguments: tc.args.to_string(),
                            },
                        })
                        .collect();
                    out.push(OpenAIMessage {
                        role: "assistant".into(),
                        content: text.clone(),
                        tool_calls: if wire_calls.is_empty() {
                            None
                        } else {
                            Some(wire_calls)
                        },
                        tool_call_id: None,
                        name: None,
                    });
                }
                CanonicalMessage::ToolResult(r) => {
                    out.push(OpenAIMessage {
                        role: "tool".into(),
                        content: Some(r.content.clone()),
                        tool_calls: None,
                        tool_call_id: Some(r.tool_call_id.clone()),
                        name: Some(r.name.clone()),
                    });
                }
            }
        }
        out
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(
        &self,
        history: &[CanonicalMessage],
        tools: &[FunctionDeclaration],
        model: &str,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<ProviderResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/chat/completions", self.base_url);

        let oai_tools: Vec<OpenAITool> = tools
            .iter()
            .map(|t| OpenAITool {
                tool_type: "function".into(),
                function: OpenAIFunctionSpec {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
            .collect();

        let request = OpenAIRequest {
            model: model.to_string(),
            messages: Self::to_wire(history),
            tools: if oai_tools.is_empty() {
                None
            } else {
                Some(oai_tools)
            },
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error ({status}): {body}").into());
        }

        let mut stream = response.bytes_stream();
        let mut text_buf = String::new();
        let mut wire_buf = String::new();
        let mut accum: Vec<(String, String, String)> = Vec::new();

        'outer: while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            wire_buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(end) = wire_buf.find('\n') {
                let line = wire_buf[..end].trim().to_string();
                wire_buf.drain(..=end);

                if line == "data: [DONE]" {
                    break 'outer;
                }

                if let Some(json) = line.strip_prefix("data: ") {
                    if let Ok(chunk) = serde_json::from_str::<OpenAIChunk>(json) {
                        for choice in chunk.choices {
                            let delta = choice.delta;
                            if let Some(text) = delta.content {
                                on_text(text.clone());
                                text_buf.push_str(&text);
                            }
                            if let Some(tc_deltas) = delta.tool_calls {
                                for d in tc_deltas {
                                    let idx = d.index;
                                    while accum.len() <= idx {
                                        accum.push(Default::default());
                                    }
                                    if let Some(id) = d.id {
                                        accum[idx].0 = id;
                                    }
                                    if let Some(f) = d.function {
                                        if let Some(name) = f.name {
                                            accum[idx].1 = name;
                                        }
                                        if let Some(args) = f.arguments {
                                            accum[idx].2.push_str(&args);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let tool_calls = accum
            .into_iter()
            .filter(|(id, name, _)| !id.is_empty() && !name.is_empty())
            .map(|(id, name, args_str)| CanonicalToolCall {
                id,
                name,
                args: serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Object(Default::default())),
                thought_signature: None,
            })
            .collect();

        Ok(ProviderResponse {
            text: text_buf,
            tool_calls,
        })
    }
}
pub struct Agent {
    provider: Arc<dyn LLMProvider>,
    pub model: String,
    history: Vec<CanonicalMessage>,
    executor: ToolExecutor,
    tools: Vec<FunctionDeclaration>,
    dynamic_tools: HashMap<String, Arc<dyn AgentTool>>,
    workspace_root: PathBuf,
}

impl Agent {
    pub fn new_gemini(api_key: String, workspace_root: PathBuf, model: String) -> Self {
        let provider = Arc::new(GeminiProvider::new(api_key, model.clone()));
        Self::with_provider(provider, model, workspace_root)
    }

    pub fn new_openai(api_key: String, workspace_root: PathBuf, model: String) -> Self {
        let provider = Arc::new(OpenAIProvider::new(api_key, model.clone()));
        Self::with_provider(provider, model, workspace_root)
    }

    pub fn set_provider(&mut self, provider: Arc<dyn LLMProvider>, model: String) {
        self.provider = provider;
        self.model = model;
    }

    pub fn with_provider(
        provider: Arc<dyn LLMProvider>,
        model: String,
        workspace_root: PathBuf,
    ) -> Self {
        let mut agent = Self {
            provider,
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

    pub fn with_tool(mut self, tool: Arc<dyn AgentTool>) -> Self {
        let decl = tool.declaration();
        self.dynamic_tools.insert(decl.name.clone(), tool);
        self.tools.push(decl);
        self
    }

    pub fn get_workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub async fn undo(&self) -> Result<String, std::io::Error> {
        self.executor.undo_git_checkpoint().await
    }

    pub fn get_history_stats(&self) -> (usize, usize) {
        let count = self.history.len();
        let chars: usize = self
            .history
            .iter()
            .map(|m| match m {
                CanonicalMessage::User(t) => t.len(),
                CanonicalMessage::Assistant { text, .. } => text.as_deref().map_or(0, |t| t.len()),
                CanonicalMessage::ToolResult(r) => r.content.len(),
            })
            .sum();
        (count, chars)
    }

    pub fn save_session(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_session(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        self.history = serde_json::from_str(&json)?;
        Ok(())
    }

    fn initialize_system_context(&mut self) {
        let project_map = self.build_project_map(&self.workspace_root.clone(), "");
        let system_prompt = format!(
            "You are Rune, an expert AI coding assistant integrated into a software development workspace.\n\
            \n\
            ## Repository Architecture & File Tree:\n\
            {project_map}\n\
            \n\
            ## Guidelines & Smart Context:\n\
            - You have global awareness of the repository architecture from the file tree above.\n\
            - When the user mentions specific files using `@filename` (e.g. `@src/api.rs`), those files are automatically loaded and injected into your prompt context.\n\
            - Use `search_code` (powered by the `cix` indexed search engine) to instantly search for functions, symbols, or patterns across the repository when you need to locate code.\n\
            - Use `read_file`, `list_files`, `write_file`, `patch_file`, `execute_commands`, and `execute_batch` as needed to inspect and modify code.\n\
            - Prefer `patch_file` over `write_file` for surgical code edits using search and replace blocks.\n\
            - Be concise, precise, and proactive."
        );

        self.history.push(CanonicalMessage::User(format!(
            "[SYSTEM ARCHITECTURE INITIALIZATION]\n{system_prompt}"
        )));
        self.history.push(CanonicalMessage::Assistant {
            text: Some(
                "Understood. I have loaded the repository architecture and am ready to assist. \
                You can `@mention` files or ask me to search and modify code."
                    .into(),
            ),
            tool_calls: vec![],
        });
    }

    pub fn build_project_map(&self, dir: &Path, prefix: &str) -> String {
        let mut output = String::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return format!("{prefix} [Error reading directory]\n"),
        };

        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        paths.sort_by(|a, b| {
            let a_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if a_dir != b_dir {
                b_dir.cmp(&a_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        let filtered: Vec<_> = paths
            .into_iter()
            .filter(|e| {
                let name = e.file_name();
                let s = name.to_string_lossy();
                s != "target" && s != ".git" && s != ".DS_Store" && !s.ends_with(".rs.bk")
            })
            .collect();

        let count = filtered.len();
        for (i, entry) in filtered.into_iter().enumerate() {
            let is_last = i == count - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let name = entry.file_name();
            let name_s = name.to_string_lossy();
            let path = entry.path();
            if path.is_dir() {
                output.push_str(&format!("{prefix}{connector}{name_s}/\n"));
                let ext = if is_last { "    " } else { "│   " };
                output.push_str(&self.build_project_map(&path, &format!("{prefix}{ext}")));
            } else {
                output.push_str(&format!("{prefix}{connector}{name_s}\n"));
            }
        }
        output
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.initialize_system_context();
    }

    /// Truncate history to keep the last `max_messages` while preserving
    /// the initial system architecture context (messages 0 and 1).
    pub fn truncate_history(&mut self, max_messages: usize) {
        if self.history.len() <= 2 {
            return;
        }
        // System context is always the first 2 messages (User init + Assistant ACK)
        let system_len = 2;
        if self.history.len() > system_len + max_messages {
            let start = self.history.len() - max_messages;
            let mut pruned = Vec::with_capacity(system_len + max_messages);
            // Retain system init context
            pruned.extend(self.history[..system_len].iter().cloned());
            // Retain the last `max_messages`
            pruned.extend(self.history[start..].iter().cloned());
            self.history = pruned;
            println!("[Rune: History truncated to last {} messages (+ system context)]", max_messages);
        }
    }

    pub async fn run(&mut self, raw_prompt: &str, auto_approve: bool) {
        let processed = self.resolve_mentions(raw_prompt).await;
        self.history.push(CanonicalMessage::User(processed));

        loop {
            let result = self
                .provider
                .complete(&self.history, &self.tools, &self.model, &|text: String| {
                    print!("{text}");
                    let _ = std::io::stdout().flush();
                })
                .await;

            let ProviderResponse { text, tool_calls } = match result {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Provider error: {e}");
                    break;
                }
            };

            self.history.push(CanonicalMessage::Assistant {
                text: if text.is_empty() { None } else { Some(text) },
                tool_calls: tool_calls.clone(),
            });

            if tool_calls.is_empty() {
                println!();
                break;
            }

            let (read_only, mutating): (Vec<_>, Vec<_>) = tool_calls
                .into_iter()
                .partition(|tc| self.is_read_only_tool(&tc.name));

            let mut tool_results: Vec<CanonicalToolResult> = Vec::new();

            if !read_only.is_empty() {
                let executor = &self.executor;
                let ro_dynamic: HashMap<String, Arc<dyn AgentTool>> = read_only
                    .iter()
                    .filter_map(|tc| {
                        self.dynamic_tools
                            .get(&tc.name)
                            .map(|t| (tc.name.clone(), Arc::clone(t)))
                    })
                    .collect();

                let futures = read_only.into_iter().map(|tc| {
                    let ro_dynamic = ro_dynamic.clone();
                    println!("\n[Executing read-only tool concurrently: {}]", tc.name);
                    async move {
                        let output = if let Some(tool) = ro_dynamic.get(&tc.name) {
                            tool.execute(tc.args.clone()).await
                        } else {
                            match tc.name.as_str() {
                                "list_files" => {
                                    let p = tc.args["path"].as_str().unwrap_or(".");
                                    executor
                                        .list_files(Path::new(p))
                                        .await
                                        .unwrap_or_else(|e| e.to_string())
                                }
                                "read_file" => {
                                    let p = tc.args["path"].as_str().unwrap_or("");
                                    executor
                                        .read_file(Path::new(p))
                                        .await
                                        .unwrap_or_else(|e| e.to_string())
                                }
                                "search_code" => {
                                    let q = tc.args["query"].as_str().unwrap_or("");
                                    executor
                                        .search_code(q)
                                        .await
                                        .unwrap_or_else(|e| e.to_string())
                                }
                                "git_status" => executor
                                    .git_status()
                                    .await
                                    .unwrap_or_else(|e| e.to_string()),
                                "git_diff" => executor
                                    .git_diff(tc.args["path"].as_str())
                                    .await
                                    .unwrap_or_else(|e| e.to_string()),
                                _ => format!("Unknown read-only tool: {}", tc.name),
                            }
                        };
                        CanonicalToolResult {
                            tool_call_id: tc.id,
                            name: tc.name,
                            content: output,
                        }
                    }
                });

                let mut ro_results = futures_util::future::join_all(futures).await;
                tool_results.append(&mut ro_results);
            }

            for tc in mutating {
                println!("\n[Executing tool sequentially: {}]", tc.name);
                let output = self.execute_tool(&tc, auto_approve).await;
                tool_results.push(CanonicalToolResult {
                    tool_call_id: tc.id,
                    name: tc.name,
                    content: output,
                });
            }

            for result in tool_results {
                self.history.push(CanonicalMessage::ToolResult(result));
            }
        }
    }

    fn is_read_only_tool(&self, name: &str) -> bool {
        if let Some(tool) = self.dynamic_tools.get(name) {
            return !tool.is_destructive();
        }
        matches!(
            name,
            "list_files" | "read_file" | "search_code" | "git_status" | "git_diff"
        )
    }

    async fn execute_tool(&self, tc: &CanonicalToolCall, auto_approve: bool) -> String {
        if let Some(dynamic_tool) = self.dynamic_tools.get(&tc.name) {
            if dynamic_tool.is_destructive() && !auto_approve && !confirm_execution(tc).await {
                println!("\n[Execution cancelled: {}]", tc.name);
                return "Tool execution rejected by user.".into();
            }
            return dynamic_tool.execute(tc.args.clone()).await;
        }

        match tc.name.as_str() {
            "patch_file" => {
                let path = tc.args["path"].as_str().unwrap_or("");
                let search = tc.args["search"].as_str().unwrap_or("");
                let replace = tc.args["replace"].as_str().unwrap_or("");
                match self
                    .executor
                    .preview_patch(Path::new(path), search, replace)
                    .await
                {
                    Ok((safe_path, new_content)) => {
                        let (sl, rl) = (search.len(), replace.len());
                        if !auto_approve && !confirm_execution(tc).await {
                            return "Tool execution rejected by user.".into();
                        }
                        self.executor
                            .apply_patch(&safe_path, &new_content, sl, rl)
                            .await
                            .map(|_| "File patched successfully.".into())
                            .unwrap_or_else(|e| format!("Error applying patch: {e}"))
                    }
                    Err(e) => format!("Error previewing patch: {e}"),
                }
            }
            "write_file" => {
                let path = tc.args["path"].as_str().unwrap_or("");
                let content = tc.args["content"].as_str().unwrap_or("");
                match self.executor.preview_write(Path::new(path), content).await {
                    Ok(safe_path) => {
                        if !auto_approve && !confirm_execution(tc).await {
                            return "Tool execution rejected by user.".into();
                        }
                        self.executor
                            .apply_write(&safe_path, content)
                            .await
                            .unwrap_or_else(|e| format!("Error writing file: {e}"))
                    }
                    Err(e) => format!("Error previewing write: {e}"),
                }
            }
            _ => {
                if !auto_approve && !confirm_execution(tc).await {
                    println!("\n[Execution cancelled: {}]", tc.name);
                    return "Tool execution rejected by user.".into();
                }
                match tc.name.as_str() {
                    "git_commit" => {
                        let msg = tc.args["message"].as_str().unwrap_or("rune update");
                        self.executor
                            .git_commit(msg)
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    }
                    "undo_git_checkpoint" => self
                        .executor
                        .undo_git_checkpoint()
                        .await
                        .unwrap_or_else(|e| format!("Error: {e}")),
                    "execute_commands" => {
                        let cmd = tc.args["cmd"].as_str().unwrap_or("");
                        self.executor
                            .execute_commands(cmd)
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    }
                    "execute_batch" => {
                        let cmds: Vec<String> = tc.args["commands"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        self.executor
                            .execute_batch(&cmds)
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    }
                    unknown => format!("Unknown tool: '{unknown}'"),
                }
            }
        }
    }

    async fn resolve_mentions(&self, raw: &str) -> String {
        let mut resolved = raw.to_string();
        let mut injections = Vec::new();

        for word in raw.split_whitespace() {
            if word.len() <= 1 || !word.starts_with('@') {
                continue;
            }
            let clean = word[1..].trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_'
            });
            if clean.is_empty() {
                continue;
            }

            match self.executor.read_file(Path::new(clean)).await {
                Ok(content) => {
                    injections.push(format!(
                        "\n\n[Auto-injected content of @{clean}]:\n```\n{content}\n```"
                    ));
                    println!("[Rune: Auto-scraped @{clean} into context]");
                }
                Err(e) => println!("[Rune: Warning: failed to read @{clean}: {e}]"),
            }
        }

        for inj in injections {
            resolved.push_str(&inj);
        }
        resolved
    }
}

async fn confirm_execution(tc: &CanonicalToolCall) -> bool {
    let tc = tc.clone();
    task::spawn_blocking(move || -> bool {
        println!("\n The agent wants to execute a potentially destructive operation:");
        println!("   Function : {}", tc.name);
        if matches!(tc.name.as_str(), "execute_commands" | "execute_batch") {
            println!("   Args     : {}", tc.args);
        }
        print!("   Allow execution? [y/N]: ");
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let t = input.trim().to_lowercase();
            t == "y" || t == "yes"
        } else {
            false
        }
    })
    .await
    .unwrap_or(false)
}

pub fn get_tool_declarations() -> Vec<FunctionDeclaration> {
    vec![
        FunctionDeclaration {
            name: "list_files".into(),
            description: "List files and directories in a given relative path.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Relative file path" } },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "read_file".into(),
            description: "Read the full text content of a file.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Global or relative file path" } },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".into(),
            description:
                "Write or overwrite content to a file (use for new files or complete rewrites)."
                    .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path" },
                    "content": { "type": "string", "description": "Text content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "patch_file".into(),
            description:
                "Surgical search-and-replace on a file. Prefer this over write_file for edits."
                    .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path" },
                    "search": { "type": "string", "description": "Exact block to find" },
                    "replace": { "type": "string", "description": "Replacement block" }
                },
                "required": ["path", "search", "replace"]
            }),
        },
        FunctionDeclaration {
            name: "search_code".into(),
            description:
                "Search the codebase via the cix indexed engine for functions, symbols, or queries."
                    .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string", "description": "Search query or symbol name" } },
                "required": ["query"]
            }),
        },
        FunctionDeclaration {
            name: "git_status".into(),
            description: "Check the current git status of the repository.".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }),
        },
        FunctionDeclaration {
            name: "git_diff".into(),
            description: "Show working-tree changes, optionally scoped to a file path.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Optional file path to diff" } },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "git_commit".into(),
            description: "Stage all changes and create a git commit.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "message": { "type": "string", "description": "Commit message" } },
                "required": ["message"]
            }),
        },
        FunctionDeclaration {
            name: "undo_git_checkpoint".into(),
            description: "Undo the last agent mutation by popping the git stash checkpoint.".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }),
        },
        FunctionDeclaration {
            name: "execute_commands".into(),
            description: "Execute a single shell command in the workspace.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "cmd": { "type": "string", "description": "Command to run (e.g. 'cargo check')" } },
                "required": ["cmd"]
            }),
        },
        FunctionDeclaration {
            name: "execute_batch".into(),
            description: "Execute a list of shell commands sequentially in the workspace.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Commands to run in order (e.g. ['cargo check', 'cargo test'])"
                    }
                },
                "required": ["commands"]
            }),
        },
    ]
}

fn gemini_normalize_schema(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(t)) = map.get_mut("type") {
                *t = t.to_uppercase();
            }
            for v in map.values_mut() {
                gemini_normalize_schema(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                gemini_normalize_schema(v);
            }
        }
        _ => {}
    }
}
