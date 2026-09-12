use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{Value, json};

pub struct HttpRequestTool {
    client: reqwest::Client,
}

impl HttpRequestTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for HttpRequestTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "http_request".to_string(),
            description: "Execute a general REST HTTP request (GET, POST, PUT, DELETE, PATCH) with optional headers and body."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "method": {
                        "type": "STRING",
                        "description": "HTTP method: GET, POST, PUT, DELETE, PATCH"
                    },
                    "url": {
                        "type": "STRING",
                        "description": "Full target URL (e.g. https://api.example.com/v1/data)"
                    },
                    "headers": {
                        "type": "OBJECT",
                        "description": "Optional JSON object containing HTTP headers (e.g. {\"Authorization\": \"Bearer token\"})"
                    },
                    "body": {
                        "type": "STRING",
                        "description": "Optional request body string (JSON or text)"
                    }
                },
                "required": ["method", "url"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false // Can be mutating / trigger webhooks
    }

    async fn execute(&self, args: Value) -> String {
        let method_str = match args.get("method").and_then(|v| v.as_str()) {
            Some(m) => m.to_uppercase(),
            None => return "Error: Missing required argument 'method'".to_string(),
        };

        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return "Error: Missing required argument 'url'".to_string(),
        };

        let method = match method_str.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            other => return format!("Error: Unsupported HTTP method '{other}'"),
        };

        let mut req_builder = self.client.request(method, url);

        if let Some(headers_obj) = args.get("headers").and_then(|v| v.as_object()) {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, val) in headers_obj {
                if let Some(val_str) = val.as_str() {
                    if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                        if let Ok(value) = reqwest::header::HeaderValue::from_str(val_str) {
                            header_map.insert(name, value);
                        }
                    }
                }
            }
            req_builder = req_builder.headers(header_map);
        }

        if let Some(body_str) = args.get("body").and_then(|v| v.as_str()) {
            if !body_str.is_empty() {
                req_builder = req_builder.body(body_str.to_string());
            }
        }

        match req_builder.send().await {
            Ok(res) => {
                let status = res.status();

                let status_line = format!("HTTP/1.1 {}\n", status);

                let headers_output: String = res
                    .headers()
                    .iter()
                    .map(|(k, v)| format!("{}: {}\n", k, v.to_str().unwrap_or("<invalid>")))
                    .collect();

                let body = match res.text().await {
                    Ok(b) => b,
                    Err(e) => format!("[Error reading response body: {e}]"),
                };

                let output = format!("{}{}\n{}", status_line, headers_output, body);
                output
            }
            Err(e) => format!("HTTP Request Network Error: {e}"),
        }
    }
}

pub struct FetchWebPageTool {
    client: reqwest::Client,
}

impl FetchWebPageTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("RuneAgent/0.1.0 (Content Scraper)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for FetchWebPageTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "fetch_web_page".to_string(),
            description: "Fetch a URL (API docs, patch notes, GitHub README, article) and extract its text content."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "url": {
                        "type": "STRING",
                        "description": "Target web URL to fetch and scrape text content from"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return "Error: Missing required argument 'url'".to_string(),
        };

        match self.client.get(url).send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    return format!("Error: HTTP status {}", res.status());
                }

                // Extract content_type before res is consumed by .text()
                let content_type = res
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();

                match res.text().await {
                    Ok(html) => {
                        if content_type.contains("html") {
                            strip_html_tags(&html)
                        } else {
                            html
                        }
                    }
                    Err(e) => format!("Error reading page body: {e}"),
                }
            }
            Err(e) => format!("Fetch Web Page Network Error: {e}"),
        }
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            in_tag = true;
            let remaining: String = chars[i..std::cmp::min(i + 10, chars.len())]
                .iter()
                .collect();
            let lower = remaining.to_lowercase();
            if lower.starts_with("<script") {
                in_script = true;
            } else if lower.starts_with("</script") {
                in_script = false;
            } else if lower.starts_with("<style") {
                in_style = true;
            } else if lower.starts_with("</style") {
                in_style = false;
            }
        } else if c == '>' {
            in_tag = false;
            i += 1;
            continue;
        }

        if !in_tag && !in_script && !in_style {
            output.push(c);
        }
        i += 1;
    }

    let lines: Vec<&str> = output
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    lines.join("\n")
}

pub struct CheckTcpPortTool;

impl CheckTcpPortTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for CheckTcpPortTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "check_tcp_port".to_string(),
            description: "Verify if a local or remote TCP port (e.g. dev server on 3000, backend on 8080, Unity Bridge on 8088) is up and listening."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "port": {
                        "type": "INTEGER",
                        "description": "TCP port number to check (e.g. 3000, 8080, 8088)"
                    },
                    "host": {
                        "type": "STRING",
                        "description": "Optional host/IP address to check (defaults to 127.0.0.1)"
                    }
                },
                "required": ["port"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let port = match args.get("port").and_then(|v| v.as_i64()) {
            Some(p) => p as u16,
            None => match args
                .get("port")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u16>().ok())
            {
                Some(p) => p,
                None => return "Error: Missing or invalid required argument 'port'".to_string(),
            },
        };

        let host = args
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1");

        let addr = format!("{}:{}", host, port);

        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(_stream)) => {
                let output = format!(
                    "Success: TCP port {} on host '{}' is UP and LISTENING.",
                    port, host
                );
                output
            }
            Ok(Err(e)) => {
                let output = format!(
                    "Closed/Refused: TCP port {} on host '{}' is NOT listening ({e}).",
                    port, host
                );
                output
            }
            Err(_) => {
                let output = format!(
                    "Timeout: Connection attempt to TCP port {} on host '{}' timed out after 3 seconds.",
                    port, host
                );
                output
            }
        }
    }
}
