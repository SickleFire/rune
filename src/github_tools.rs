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

        format!("{}{}", issue_text, comments_text)
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
                            format!("Pull Request #{} has an empty diff or no changes.", pull_number)
                        } else {
                            // Truncate if excessively large to keep LLM context clean, or return full diff
                            let max_len = 65536;
                            if diff.len() > max_len {
                                format!("=== GitHub Pull Request #{} Diff (Truncated to {} chars) ===\n\n{}", pull_number, max_len, &diff[..max_len])
                            } else {
                                format!("=== GitHub Pull Request #{} Diff ===\n\n{}", pull_number, diff)
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
}
