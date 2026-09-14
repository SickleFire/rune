use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{Value, json};

pub struct GitHubIssueTool {
    client: reqwest::Client,
}

impl GitHubIssueTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("RuneAgent/0.1.0 (GitHub Integrations)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for GitHubIssueTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "github_read_issue".to_string(),
            description: "Read a GitHub issue or pull request details and comments from a repository (e.g. owner='owner', repo='repo', issue_number=42)."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "owner": {
                        "type": "STRING",
                        "description": "GitHub repository owner/organization (e.g. 'octocat')"
                    },
                    "repo": {
                        "type": "STRING",
                        "description": "GitHub repository name (e.g. 'Spoon-Knife')"
                    },
                    "issue_number": {
                        "type": "INTEGER",
                        "description": "Issue or pull request number"
                    },
                    "github_token": {
                        "type": "STRING",
                        "description": "Optional GitHub personal access token for private repos or higher rate limits"
                    }
                },
                "required": ["owner", "repo", "issue_number"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return "Error: Missing required argument 'owner'".to_string(),
        };
        let repo = match args.get("repo").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: Missing required argument 'repo'".to_string(),
        };
        let issue_number = match args.get("issue_number").and_then(|v| v.as_i64()) {
            Some(n) => n,
            None => match args.get("issue_number").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()) {
                Some(n) => n,
                None => return "Error: Missing or invalid required argument 'issue_number'".to_string(),
            },
        };

        let token = args.get("github_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let url = format!("https://api.github.com/repos/{}/{}/issues/{}", owner, repo, issue_number);
        let comments_url = format!("https://api.github.com/repos/{}/{}/issues/{}/comments", owner, repo, issue_number);

        let mut req_builder = self.client.get(&url);
        let mut comments_req_builder = self.client.get(&comments_url);

        if let Some(ref t) = token {
            let auth_val = format!("Bearer {}", t);
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                req_builder = req_builder.header(reqwest::header::AUTHORIZATION, val.clone());
                comments_req_builder = comments_req_builder.header(reqwest::header::AUTHORIZATION, val);
            }
        }

        // Fetch issue details and comments concurrently
        let issue_res = req_builder.send().await;
        let comments_res = comments_req_builder.send().await;

        let issue_text = match issue_res {
            Ok(res) => {
                if !res.status().is_success() {
                    return format!("GitHub API Error: HTTP status {} when fetching issue #{}", res.status(), issue_number);
                }
                match res.json::<Value>().await {
                    Ok(json) => {
                        let title = json.get("title").and_then(|v| v.as_str()).unwrap_or("<no title>");
                        let user = json.get("user").and_then(|v| v.as_object()).and_then(|u| u.get("login")).and_then(|v| v.as_str()).unwrap_or("unknown");
                        let state = json.get("state").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let body = json.get("body").and_then(|v| v.as_str()).unwrap_or("<no description>");
                        let html_url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                        let created_at = json.get("created_at").and_then(|v| v.as_str()).unwrap_or("");

                        format!(
                            "=== GitHub Issue #{} ({}) ===\nTitle: {}\nAuthor: {}\nCreated At: {}\nURL: {}\n\nDescription:\n{}\n",
                            issue_number, state, title, user, created_at, html_url, body
                        )
                    }
                    Err(e) => format!("Error parsing issue JSON: {e}"),
                }
            }
            Err(e) => return format!("GitHub Network Error: {e}"),
        };

        let comments_text = match comments_res {
            Ok(res) if res.status().is_success() => {
                match res.json::<Value>().await {
                    Ok(Value::Array(comments)) if !comments.is_empty() => {
                        let mut out = String::from("\n=== Comments ===\n");
                        for c in comments {
                            let commenter = c.get("user").and_then(|v| v.as_object()).and_then(|u| u.get("login")).and_then(|v| v.as_str()).unwrap_or("unknown");
                            let cbody = c.get("body").and_then(|v| v.as_str()).unwrap_or("");
                            let cdate = c.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                            out.push_str(&format!("@{} (on {}):\n{}\n\n", commenter, cdate, cbody));
                        }
                        out
                    }
                    _ => String::from("\n=== Comments ===\nNo comments found.\n"),
                }
            }
            _ => String::new(),
        };

        let out = format!("{}{}", issue_text, comments_text);
        out
    }
}

pub struct GitHubPullRequestDiffTool {
    client: reqwest::Client,
}

impl GitHubPullRequestDiffTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("RuneAgent/0.1.0 (GitHub Integrations)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for GitHubPullRequestDiffTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "github_read_pr_diff".to_string(),
            description: "Fetch the unified git diff of a GitHub pull request to assist with code reviews and PR analysis."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "owner": {
                        "type": "STRING",
                        "description": "GitHub repository owner/organization (e.g. 'octocat')"
                    },
                    "repo": {
                        "type": "STRING",
                        "description": "GitHub repository name (e.g. 'Spoon-Knife')"
                    },
                    "pull_number": {
                        "type": "INTEGER",
                        "description": "Pull request number"
                    },
                    "github_token": {
                        "type": "STRING",
                        "description": "Optional GitHub personal access token for private repos or higher rate limits"
                    }
                },
                "required": ["owner", "repo", "pull_number"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return "Error: Missing required argument 'owner'".to_string(),
        };
        let repo = match args.get("repo").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: Missing required argument 'repo'".to_string(),
        };
        let pull_number = match args.get("pull_number").and_then(|v| v.as_i64()) {
            Some(n) => n,
            None => match args.get("pull_number").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()) {
                Some(n) => n,
                None => return "Error: Missing or invalid required argument 'pull_number'".to_string(),
            },
        };

        let token = args.get("github_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let url = format!("https://api.github.com/repos/{}/{}/pulls/{}", owner, repo, pull_number);

        let mut req_builder = self.client.get(&url);
        if let Some(ref t) = token {
            let auth_val = format!("Bearer {}", t);
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                req_builder = req_builder.header(reqwest::header::AUTHORIZATION, val);
            }
        }
        // GitHub API requires application/vnd.github.v3.diff accept header to get the diff format
        req_builder = req_builder.header(reqwest::header::ACCEPT, "application/vnd.github.v3.diff");

        match req_builder.send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    return format!("GitHub API Error: HTTP status {} when fetching PR diff for #{}", res.status(), pull_number);
                }
                match res.text().await {
                    Ok(diff) => {
                        if diff.is_empty() {
                            let out = format!("Pull Request #{} has an empty diff or no changes.", pull_number);
                            out
                        } else {
                            // Truncate if excessively large to keep LLM context clean, or return full diff
                            let max_len = 65536;
                            if diff.len() > max_len {
                                let out = format!("=== GitHub Pull Request #{} Diff (Truncated to {} chars) ===\n\n{}", pull_number, max_len, &diff[..max_len]);
                                out
                            } else {
                                let out = format!("=== GitHub Pull Request #{} Diff ===\n\n{}", pull_number, diff);
                                out
                            }
                        }
                    }
                    Err(e) => format!("Error reading PR diff response: {e}"),
                }
            }
            Err(e) => format!("GitHub Network Error: {e}"),
        }
    }
}

pub struct GitHubCreateIssueTool {
    client: reqwest::Client,
}

impl GitHubCreateIssueTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("RuneAgent/0.1.0 (GitHub Integrations)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for GitHubCreateIssueTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "github_create_issue".to_string(),
            description: "Create a new GitHub issue in a repository with a title, body, and optional labels or assignees."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "owner": {
                        "type": "STRING",
                        "description": "GitHub repository owner/organization (e.g. 'octocat')"
                    },
                    "repo": {
                        "type": "STRING",
                        "description": "GitHub repository name (e.g. 'Spoon-Knife')"
                    },
                    "title": {
                        "type": "STRING",
                        "description": "Issue title"
                    },
                    "body": {
                        "type": "STRING",
                        "description": "Issue description / body content"
                    },
                    "labels": {
                        "type": "ARRAY",
                        "items": { "type": "STRING" },
                        "description": "Optional list of label names"
                    },
                    "github_token": {
                        "type": "STRING",
                        "description": "Optional GitHub personal access token (requires repo scope)"
                    }
                },
                "required": ["owner", "repo", "title"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return "Error: Missing required argument 'owner'".to_string(),
        };
        let repo = match args.get("repo").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: Missing required argument 'repo'".to_string(),
        };
        let title = match args.get("title").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return "Error: Missing required argument 'title'".to_string(),
        };

        let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let labels = args.get("labels").cloned().unwrap_or(json!([]));

        let token = args.get("github_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let mut req_builder = self.client.post(&url);

        if let Some(ref t) = token {
            let auth_val = format!("Bearer {}", t);
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                req_builder = req_builder.header(reqwest::header::AUTHORIZATION, val);
            }
        }

        let payload = json!({
            "title": title,
            "body": body,
            "labels": labels
        });

        match req_builder.json(&payload).send().await {
            Ok(res) => {
                let status = res.status();
                match res.json::<Value>().await {
                    Ok(json) => {
                        if status.is_success() {
                            let number = json.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                            let html_url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                            let out = format!("Successfully created GitHub issue #{}! URL: {}", number, html_url);
                            out
                        } else {
                            let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                            let out = format!("GitHub API Error (HTTP {}): {}", status, message);
                            out
                        }
                    }
                    Err(e) => format!("GitHub API Error (HTTP {}), failed to parse JSON response: {e}", status),
                }
            }
            Err(e) => format!("GitHub Network Error: {e}"),
        }
    }
}

pub struct GitHubCreateCommentTool {
    client: reqwest::Client,
}

impl GitHubCreateCommentTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("RuneAgent/0.1.0 (GitHub Integrations)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for GitHubCreateCommentTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "github_create_comment".to_string(),
            description: "Post a comment on an existing GitHub issue or pull request."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "owner": {
                        "type": "STRING",
                        "description": "GitHub repository owner/organization (e.g. 'octocat')"
                    },
                    "repo": {
                        "type": "STRING",
                        "description": "GitHub repository name (e.g. 'Spoon-Knife')"
                    },
                    "issue_number": {
                        "type": "INTEGER",
                        "description": "Issue or pull request number"
                    },
                    "body": {
                        "type": "STRING",
                        "description": "Comment markdown body"
                    },
                    "github_token": {
                        "type": "STRING",
                        "description": "Optional GitHub personal access token"
                    }
                },
                "required": ["owner", "repo", "issue_number", "body"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return "Error: Missing required argument 'owner'".to_string(),
        };
        let repo = match args.get("repo").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: Missing required argument 'repo'".to_string(),
        };
        let issue_number = match args.get("issue_number").and_then(|v| v.as_i64()) {
            Some(n) => n,
            None => match args.get("issue_number").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()) {
                Some(n) => n,
                None => return "Error: Missing or invalid required argument 'issue_number'".to_string(),
            },
        };
        let body = match args.get("body").and_then(|v| v.as_str()) {
            Some(b) => b,
            None => return "Error: Missing required argument 'body'".to_string(),
        };

        let token = args.get("github_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let url = format!("https://api.github.com/repos/{}/{}/issues/{}/comments", owner, repo, issue_number);
        let mut req_builder = self.client.post(&url);

        if let Some(ref t) = token {
            let auth_val = format!("Bearer {}", t);
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                req_builder = req_builder.header(reqwest::header::AUTHORIZATION, val);
            }
        }

        let payload = json!({ "body": body });

        match req_builder.json(&payload).send().await {
            Ok(res) => {
                let status = res.status();
                match res.json::<Value>().await {
                    Ok(json) => {
                        if status.is_success() {
                            let html_url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                            let out = format!("Successfully posted comment on issue/PR #{}! URL: {}", issue_number, html_url);
                            out
                        } else {
                            let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                            let out = format!("GitHub API Error (HTTP {}): {}", status, message);
                            out
                        }
                    }
                    Err(e) => format!("GitHub API Error (HTTP {}), failed to parse JSON response: {e}", status),
                }
            }
            Err(e) => format!("GitHub Network Error: {e}"),
        }
    }
}

pub struct GitHubCreatePullRequestTool {
    client: reqwest::Client,
}

impl GitHubCreatePullRequestTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("RuneAgent/0.1.0 (GitHub Integrations)")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for GitHubCreatePullRequestTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "github_create_pull_request".to_string(),
            description: "Create a new GitHub pull request from a head branch into a base branch."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "owner": {
                        "type": "STRING",
                        "description": "GitHub repository owner/organization (e.g. 'octocat')"
                    },
                    "repo": {
                        "type": "STRING",
                        "description": "GitHub repository name (e.g. 'Spoon-Knife')"
                    },
                    "title": {
                        "type": "STRING",
                        "description": "Pull request title"
                    },
                    "head": {
                        "type": "STRING",
                        "description": "The name of the branch where your changes are implemented (e.g. 'feature-branch')"
                    },
                    "base": {
                        "type": "STRING",
                        "description": "The name of the branch you want the changes pulled into (e.g. 'main')"
                    },
                    "body": {
                        "type": "STRING",
                        "description": "Optional pull request description"
                    },
                    "draft": {
                        "type": "BOOLEAN",
                        "description": "Whether to create the pull request as a draft (default false)"
                    },
                    "github_token": {
                        "type": "STRING",
                        "description": "Optional GitHub personal access token"
                    }
                },
                "required": ["owner", "repo", "title", "head", "base"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let owner = match args.get("owner").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return "Error: Missing required argument 'owner'".to_string(),
        };
        let repo = match args.get("repo").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: Missing required argument 'repo'".to_string(),
        };
        let title = match args.get("title").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return "Error: Missing required argument 'title'".to_string(),
        };
        let head = match args.get("head").and_then(|v| v.as_str()) {
            Some(h) => h,
            None => return "Error: Missing required argument 'head'".to_string(),
        };
        let base = match args.get("base").and_then(|v| v.as_str()) {
            Some(b) => b,
            None => return "Error: Missing required argument 'base'".to_string(),
        };

        let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);

        let token = args.get("github_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok());

        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
        let mut req_builder = self.client.post(&url);

        if let Some(ref t) = token {
            let auth_val = format!("Bearer {}", t);
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&auth_val) {
                req_builder = req_builder.header(reqwest::header::AUTHORIZATION, val);
            }
        }

        let payload = json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body,
            "draft": draft
        });

        match req_builder.json(&payload).send().await {
            Ok(res) => {
                let status = res.status();
                match res.json::<Value>().await {
                    Ok(json) => {
                        if status.is_success() {
                            let number = json.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
                            let html_url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                            let out = format!("Successfully created GitHub Pull Request #{}! URL: {}", number, html_url);
                            out
                        } else {
                            let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                            let out = format!("GitHub API Error (HTTP {}): {}", status, message);
                            out
                        }
                    }
                    Err(e) => format!("GitHub API Error (HTTP {}), failed to parse JSON response: {e}", status),
                }
            }
            Err(e) => format!("GitHub Network Error: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_github_issue_missing_args() {
        let tool = GitHubIssueTool::new();
        let res = tool.execute(json!({ "owner": "test" })).await;
        assert!(res.contains("Error: Missing required argument 'repo'"));

        let res2 = tool.execute(json!({ "owner": "test", "repo": "repo" })).await;
        assert!(res2.contains("Error: Missing or invalid required argument 'issue_number'"));
    }

    #[tokio::test]
    async fn test_github_pr_diff_missing_args() {
        let tool = GitHubPullRequestDiffTool::new();
        let res = tool.execute(json!({ "owner": "test", "repo": "repo" })).await;
        assert!(res.contains("Error: Missing or invalid required argument 'pull_number'"));
    }

    #[tokio::test]
    async fn test_github_create_issue_missing_args() {
        let tool = GitHubCreateIssueTool::new();
        let res = tool.execute(json!({ "owner": "test" })).await;
        assert!(res.contains("Error: Missing required argument 'repo'"));

        let res2 = tool.execute(json!({ "owner": "test", "repo": "repo" })).await;
        assert!(res2.contains("Error: Missing required argument 'title'"));
    }

    #[tokio::test]
    async fn test_github_create_comment_missing_args() {
        let tool = GitHubCreateCommentTool::new();
        let res = tool.execute(json!({ "owner": "test", "repo": "repo" })).await;
        assert!(res.contains("Error: Missing or invalid required argument 'issue_number'"));

        let res2 = tool.execute(json!({ "owner": "test", "repo": "repo", "issue_number": 1 })).await;
        assert!(res2.contains("Error: Missing required argument 'body'"));
    }

    #[tokio::test]
    async fn test_github_create_pr_missing_args() {
        let tool = GitHubCreatePullRequestTool::new();
        let res = tool.execute(json!({ "owner": "test", "repo": "repo" })).await;
        assert!(res.contains("Error: Missing required argument 'title'"));

        let res2 = tool.execute(json!({ "owner": "test", "repo": "repo", "title": "feat", "head": "feature" })).await;
        assert!(res2.contains("Error: Missing required argument 'base'"));
    }
}
