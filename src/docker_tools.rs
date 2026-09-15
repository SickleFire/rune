use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;

pub struct DockerListContainersTool;

impl DockerListContainersTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerListContainersTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_list_containers".to_string(),
            description: "List Docker containers (running and/or stopped) using `docker ps`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "all": {
                        "type": "BOOLEAN",
                        "description": "If true, list all containers (default shows just running)"
                    }
                },
                "required": []
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let mut cmd = Command::new("docker");
        cmd.arg("ps");

        if args.get("all").and_then(|v| v.as_bool()).unwrap_or(false) {
            cmd.arg("-a");
        }

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    if stdout.trim().is_empty() {
                        "No Docker containers found.".to_string()
                    } else {
                        stdout.to_string()
                    }
                } else {
                    format!("Docker Error (Exit Code {}):\n{}", output.status, stderr)
                }
            }
            Err(e) => format!("Failed to execute 'docker ps': {e}. Ensure Docker CLI is installed and running."),
        }
    }
}

pub struct DockerContainerLogsTool;

impl DockerContainerLogsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerContainerLogsTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_container_logs".to_string(),
            description: "Fetch logs from a Docker container using `docker logs`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "container": {
                        "type": "STRING",
                        "description": "Container name or ID"
                    },
                    "tail": {
                        "type": "INTEGER",
                        "description": "Number of lines to show from the end of the logs (default all)"
                    }
                },
                "required": ["container"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let container = match args.get("container").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: Missing required argument 'container'".to_string(),
        };

        let mut cmd = Command::new("docker");
        cmd.arg("logs");

        if let Some(tail) = args.get("tail").and_then(|v| v.as_i64()) {
            cmd.arg("--tail").arg(tail.to_string());
        }

        cmd.arg(container);

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                // docker logs often prints container stdout/stderr to stderr or stdout
                let combined = format!("{}{}", stdout, stderr);
                if output.status.success() {
                    if combined.trim().is_empty() {
                        format!("Container '{}' logs are empty.", container)
                    } else {
                        combined
                    }
                } else {
                    format!("Docker Logs Error (Exit Code {}):\n{}", output.status, combined)
                }
            }
            Err(e) => format!("Failed to execute 'docker logs': {e}"),
        }
    }
}

pub struct DockerStartContainerTool;

impl DockerStartContainerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerStartContainerTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_start_container".to_string(),
            description: "Start one or more stopped Docker containers using `docker start`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "container": {
                        "type": "STRING",
                        "description": "Container name or ID"
                    }
                },
                "required": ["container"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let container = match args.get("container").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: Missing required argument 'container'".to_string(),
        };

        match Command::new("docker").args(["start", container]).output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    format!("Successfully started container '{}'.\n{}", container, stdout)
                } else {
                    format!("Docker Start Error (Exit Code {}):\n{}", output.status, stderr)
                }
            }
            Err(e) => format!("Failed to execute 'docker start': {e}"),
        }
    }
}

pub struct DockerStopContainerTool;

impl DockerStopContainerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerStopContainerTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_stop_container".to_string(),
            description: "Stop a running Docker container using `docker stop`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "container": {
                        "type": "STRING",
                        "description": "Container name or ID"
                    },
                    "timeout": {
                        "type": "INTEGER",
                        "description": "Seconds to wait for stop before killing (optional)"
                    }
                },
                "required": ["container"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let container = match args.get("container").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: Missing required argument 'container'".to_string(),
        };

        let mut cmd = Command::new("docker");
        cmd.arg("stop");

        if let Some(t) = args.get("timeout").and_then(|v| v.as_i64()) {
            cmd.arg("-t").arg(t.to_string());
        }

        cmd.arg(container);

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    format!("Successfully stopped container '{}'.\n{}", container, stdout)
                } else {
                    format!("Docker Stop Error (Exit Code {}):\n{}", output.status, stderr)
                }
            }
            Err(e) => format!("Failed to execute 'docker stop': {e}"),
        }
    }
}

pub struct DockerListImagesTool;

impl DockerListImagesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerListImagesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_list_images".to_string(),
            description: "List local Docker images using `docker images`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {},
                "required": []
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, _args: Value) -> String {
        match Command::new("docker").args(["images"]).output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    if stdout.trim().is_empty() {
                        "No Docker images found.".to_string()
                    } else {
                        stdout.to_string()
                    }
                } else {
                    format!("Docker Images Error (Exit Code {}):\n{}", output.status, stderr)
                }
            }
            Err(e) => format!("Failed to execute 'docker images': {e}"),
        }
    }
}

pub struct DockerInspectContainerTool;

impl DockerInspectContainerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentTool for DockerInspectContainerTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "docker_inspect_container".to_string(),
            description: "Return low-level configuration and state information for a Docker container using `docker inspect`.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "container": {
                        "type": "STRING",
                        "description": "Container name or ID"
                    }
                },
                "required": ["container"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let container = match args.get("container").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: Missing required argument 'container'".to_string(),
        };

        match Command::new("docker").args(["inspect", container]).output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    stdout.to_string()
                } else {
                    format!("Docker Inspect Error (Exit Code {}):\n{}", output.status, stderr)
                }
            }
            Err(e) => format!("Failed to execute 'docker inspect': {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_tools_missing_args() {
        let logs_tool = DockerContainerLogsTool::new();
        let res = logs_tool.execute(json!({})).await;
        assert!(res.contains("Error: Missing required argument 'container'"));

        let start_tool = DockerStartContainerTool::new();
        let res2 = start_tool.execute(json!({})).await;
        assert!(res2.contains("Error: Missing required argument 'container'"));

        let stop_tool = DockerStopContainerTool::new();
        let res3 = stop_tool.execute(json!({})).await;
        assert!(res3.contains("Error: Missing required argument 'container'"));

        let inspect_tool = DockerInspectContainerTool::new();
        let res4 = inspect_tool.execute(json!({})).await;
        assert!(res4.contains("Error: Missing required argument 'container'"));
    }

    #[tokio::test]
    async fn test_docker_tool_declarations_and_properties() {
        // Validate declarations and is_read_only properties across all docker tools
        let t1 = DockerListContainersTool::new();
        assert_eq!(t1.declaration().name, "docker_list_containers");
        assert!(t1.is_read_only());

        let t2 = DockerContainerLogsTool::new();
        assert_eq!(t2.declaration().name, "docker_container_logs");
        assert!(t2.is_read_only());

        let t3 = DockerStartContainerTool::new();
        assert_eq!(t3.declaration().name, "docker_start_container");
        assert!(!t3.is_read_only());

        let t4 = DockerStopContainerTool::new();
        assert_eq!(t4.declaration().name, "docker_stop_container");
        assert!(!t4.is_read_only());

        let t5 = DockerListImagesTool::new();
        assert_eq!(t5.declaration().name, "docker_list_images");
        assert!(t5.is_read_only());

        let t6 = DockerInspectContainerTool::new();
        assert_eq!(t6.declaration().name, "docker_inspect_container");
        assert!(t6.is_read_only());
    }
}
