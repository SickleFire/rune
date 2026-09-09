use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{Value, json};

const BRIDGE_URL: &str = "http://localhost:8088";

pub struct UnityInspectSceneTool {
    client: reqwest::Client,
}

impl UnityInspectSceneTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for UnityInspectSceneTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_inspect_scene".to_string(),
            description:
                "Inspect active Unity Editor scene hierarchy, returning GameObjects and components."
                    .to_string(),
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
        let url = format!("{}/scene/inspect", BRIDGE_URL);
        match self.client.get(&url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnitySetPropertyTool {
    client: reqwest::Client,
}

impl UnitySetPropertyTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for UnitySetPropertyTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_set_property".to_string(),
            description:
                "Modify a serialized property on a GameObject component in the active Unity scene."
                    .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "objectName": { "type": "STRING", "description": "Target GameObject name" },
                    "componentType": { "type": "STRING", "description": "Component or script class name" },
                    "propertyName": { "type": "STRING", "description": "Serialized field name" },
                    "value": { "type": "STRING", "description": "Value to set serialized as string" }
                },
                "required": ["objectName", "componentType", "propertyName", "value"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/object/set-property", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityInspectComponentsTool {
    client: reqwest::Client,
}

impl UnityInspectComponentsTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for UnityInspectComponentsTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_inspect_components".to_string(),
            description:
                "Inspect detailed fields, types, values, and object reference status for all components on a GameObject."
                    .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "objectName": { "type": "STRING", "description": "Target GameObject name to inspect" }
                },
                "required": ["objectName"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let object_name = match args.get("objectName").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return "Error: Missing required argument 'objectName'".to_string(),
        };

        let url = format!("{}/object/inspect-components", BRIDGE_URL);
        match self
            .client
            .get(&url)
            .query(&[("objectName", object_name)])
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityValidateReferencesTool {
    client: reqwest::Client,
}

impl UnityValidateReferencesTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for UnityValidateReferencesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_validate_references".to_string(),
            description:
                "Audit the active Unity scene for unassigned (null) script field references."
                    .to_string(),
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
        let url = format!("{}/scene/validate-references", BRIDGE_URL);
        match self.client.get(&url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityAssignReferenceTool {
    client: reqwest::Client,
}

impl UnityAssignReferenceTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for UnityAssignReferenceTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_assign_reference".to_string(),
            description:
                "Assign a GameObject or Component reference to a field on a script component in Unity."
                    .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "objectName": { "type": "STRING", "description": "Target GameObject holding the component" },
                    "componentType": { "type": "STRING", "description": "Script or component class name receiving the assignment" },
                    "fieldName": { "type": "STRING", "description": "Serialized object reference field name" },
                    "targetObjectName": { "type": "STRING", "description": "GameObject to assign as reference" },
                    "targetComponentType": { "type": "STRING", "description": "Optional: exact Component type on target object to bind" }
                },
                "required": ["objectName", "componentType", "fieldName", "targetObjectName"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/object/assign-reference", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityCreateGameObjectTool {
    client: reqwest::Client,
}
impl UnityCreateGameObjectTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityCreateGameObjectTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_create_game_object".to_string(),
            description: "Create a new GameObject or primitive in the active Unity scene."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "name": { "type": "STRING", "description": "Name of the new GameObject" },
                    "primitiveType": { "type": "STRING", "description": "Optional primitive type: Cube, Sphere, Capsule, Cylinder, Plane, Quad" }
                },
                "required": ["name"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }
    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/object/create", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityAddComponentTool {
    client: reqwest::Client,
}
impl UnityAddComponentTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityAddComponentTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_add_component".to_string(),
            description: "Attach a component or script to an existing GameObject in Unity."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "objectName": { "type": "STRING", "description": "Target GameObject name" },
                    "componentType": { "type": "STRING", "description": "Component or C# script class name" }
                },
                "required": ["objectName", "componentType"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }
    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/object/add-component", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityDestroyGameObjectTool {
    client: reqwest::Client,
}
impl UnityDestroyGameObjectTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityDestroyGameObjectTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_destroy_game_object".to_string(),
            description: "Remove and destroy a GameObject from the active Unity scene.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "objectName": { "type": "STRING", "description": "GameObject name to destroy" }
                },
                "required": ["objectName"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        true
    }
    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/object/destroy", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityInstantiatePrefabTool {
    client: reqwest::Client,
}
impl UnityInstantiatePrefabTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityInstantiatePrefabTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_instantiate_prefab".to_string(),
            description: "Instantiate a prefab asset into the active scene by asset path."
                .to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "prefabPath": { "type": "STRING", "description": "Path to prefab asset (e.g. Assets/Prefabs/Player.prefab)" }
                },
                "required": ["prefabPath"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    fn is_destructive(&self) -> bool {
        false
    }
    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/prefab/instantiate", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityFindAssetsTool {
    client: reqwest::Client,
}
impl UnityFindAssetsTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityFindAssetsTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_find_assets".to_string(),
            description: "Search Unity AssetDatabase using search filters (e.g. 't:Prefab', 't:ScriptableObject').".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "filter": { "type": "STRING", "description": "AssetDatabase search filter string" }
                },
                "required": ["filter"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        true
    }
    async fn execute(&self, args: Value) -> String {
        let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("");
        let url = format!("{}/asset/find", BRIDGE_URL);
        match self
            .client
            .get(&url)
            .query(&[("filter", filter)])
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityRefreshAssetDatabaseTool {
    client: reqwest::Client,
}
impl UnityRefreshAssetDatabaseTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityRefreshAssetDatabaseTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_refresh_asset_database".to_string(),
            description: "Trigger AssetDatabase.Refresh() in Unity to recompile newly added/modified scripts.".to_string(),
            parameters: json!({ "type": "OBJECT", "properties": {}, "required": [] }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    async fn execute(&self, _args: Value) -> String {
        let url = format!("{}/asset/refresh", BRIDGE_URL);
        match self.client.post(&url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityReadConsoleLogsTool {
    client: reqwest::Client,
}
impl UnityReadConsoleLogsTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityReadConsoleLogsTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_read_console_logs".to_string(),
            description: "Retrieve recent Unity Editor console logs and script compilation errors."
                .to_string(),
            parameters: json!({ "type": "OBJECT", "properties": {}, "required": [] }),
        }
    }
    fn is_read_only(&self) -> bool {
        true
    }
    async fn execute(&self, _args: Value) -> String {
        let url = format!("{}/console/logs", BRIDGE_URL);
        match self.client.get(&url).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}

pub struct UnityTogglePlayModeTool {
    client: reqwest::Client,
}
impl UnityTogglePlayModeTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}
#[async_trait]
impl AgentTool for UnityTogglePlayModeTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "unity_toggle_play_mode".to_string(),
            description: "Control Unity Editor Play Mode state (play, stop, pause).".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "state": { "type": "STRING", "description": "Desired state: 'play', 'stop', or 'pause'" }
                },
                "required": ["state"]
            }),
        }
    }
    fn is_read_only(&self) -> bool {
        false
    }
    async fn execute(&self, args: Value) -> String {
        let url = format!("{}/editor/play-mode", BRIDGE_URL);
        match self.client.post(&url).json(&args).send().await {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) => format!("Unity Bridge Error: HTTP {}", res.status()),
            Err(e) => format!("Unity Bridge Network Error: {e}"),
        }
    }
}
