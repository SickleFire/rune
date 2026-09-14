use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct GodotInspectSceneTool {
    client: reqwest::Client,
}

impl GodotInspectSceneTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AgentTool for GodotInspectSceneTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "godot_inspect_scene".to_string(),
            description: "Inspect active Godot Editor scene hierarchy, returning root nodes and their children.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _args: Value) -> String {
        let url = "http://localhost:8089/scene/inspect";
        match self.client.get(url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Godot Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Godot Bridge Network Error: {e} (Ensure Godot editor is running with RuneBridge.cs active on port 8089)"),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

pub struct GodotInspectNodePropertiesTool {
    client: reqwest::Client,
}

impl GodotInspectNodePropertiesTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AgentTool for GodotInspectNodePropertiesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "godot_inspect_node_properties".to_string(),
            description: "Inspect properties and exported fields of a Godot node by its path.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "nodePath": {
                        "type": "STRING",
                        "description": "The path or name of the Godot node to inspect (e.g. '/root/Main/Player')."
                    }
                },
                "required": ["nodePath"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> String {
        let node_path = match args.get("nodePath").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: Missing 'nodePath' argument".to_string(),
        };

        let url = format!("http://localhost:8089/node/inspect-properties?nodePath={}", urlencoding::encode(node_path));
        match self.client.get(&url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Godot Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Godot Bridge Network Error: {e}"),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

pub struct GodotCreateNodeTool {
    client: reqwest::Client,
}

impl GodotCreateNodeTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AgentTool for GodotCreateNodeTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "godot_create_node".to_string(),
            description: "Create and add a new node to the active Godot scene.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "parentPath": {
                        "type": "STRING",
                        "description": "The path of the parent node."
                    },
                    "nodeName": {
                        "type": "STRING",
                        "description": "Name of the new node."
                    },
                    "nodeType": {
                        "type": "STRING",
                        "description": "Type of node to create (e.g. 'Node', 'Sprite2D', 'RigidBody2D')."
                    }
                },
                "required": ["parentPath", "nodeName", "nodeType"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> String {
        let url = "http://localhost:8089/node/create";
        match self.client.post(url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Godot Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Godot Bridge Network Error: {e}"),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        false
    }
}
