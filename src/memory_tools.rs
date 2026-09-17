use crate::api::FunctionDeclaration;
use crate::memory::MemoryStore;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::json;

pub struct RememberPreferenceTool;

#[async_trait]
impl AgentTool for RememberPreferenceTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "remember_preference".to_string(),
            description: "Store a persistent user preference or repo convention (e.g., coding style, verbosity, default tooling) in Rune's memory store.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "key": {
                        "type": "STRING",
                        "description": "Preference key name (e.g. 'coding_style', 'preferred_framework')."
                    },
                    "value": {
                        "type": "STRING",
                        "description": "Preference value or description."
                    }
                },
                "required": ["key", "value"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let key = match args.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => return "Error: 'key' parameter is required.".to_string(),
        };
        let value = match args.get("value").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return "Error: 'value' parameter is required.".to_string(),
        };

        let mut store = MemoryStore::load();
        match store.set_preference(key, value) {
            Ok(()) => format!("Successfully stored preference '{}' = '{}'.", key, value),
            Err(e) => format!("Failed to save memory: {}", e),
        }
    }
}

pub struct RecallMemoryTool;

#[async_trait]
impl AgentTool for RecallMemoryTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "recall_memory".to_string(),
            description: "Search Rune's persistent memory store for past fixes, user preferences, or file & symbol relationships.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "query": {
                        "type": "STRING",
                        "description": "Search query or keyword (e.g. error message, symbol name, preference key)."
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return "Error: 'query' parameter is required.".to_string(),
        };

        let store = MemoryStore::load();
        let fixes = store.search_fixes(query);
        let relations = store.query_relations(query);

        let mut output = String::new();
        output.push_str(&format!("Memory search results for '{}':\n\n", query));

        // Preferences match
        if let Some(pref) = store.get_preference(query) {
            output.push_str(&format!("[Preference] {} = {}\n", query, pref));
        }
        let q_lower = query.to_lowercase();
        let matching_prefs: Vec<(&String, &String)> = store
            .preferences
            .iter()
            .filter(|(k, v)| {
                (k.to_lowercase().contains(&q_lower) || v.to_lowercase().contains(&q_lower))
                    && *k != query
            })
            .collect();
        if !matching_prefs.is_empty() {
            output.push_str("--- Preferences ---\n");
            for (k, v) in matching_prefs {
                output.push_str(&format!("{} = {}\n", k, v));
            }
        }

        if !fixes.is_empty() {
            output.push_str("--- Past Fixes ---\n");
            for fix in fixes {
                output.push_str(&format!(
                    "Problem: {}\nSolution: {}\n\n",
                    fix.problem, fix.solution
                ));
            }
        }

        if !relations.is_empty() {
            output.push_str("--- File & Symbol Relations ---\n");
            for rel in relations {
                output.push_str(&format!(
                    "File: {} -> Symbol: [{}] {} -> Target: {} (Notes: {})\n",
                    rel.file_path,
                    rel.relation_type,
                    rel.related_symbol,
                    rel.target_file,
                    rel.notes
                ));
            }
        }

        if output.len() == format!("Memory search results for '{}':\n\n", query).len() {
            output.push_str("No matching memories found.");
        }

        output
    }
}

pub struct RecordFixTool;

#[async_trait]
impl AgentTool for RecordFixTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "record_fix".to_string(),
            description: "Record a successful bug fix, error signature, and solution in Rune's persistent memory for future recall.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "problem": {
                        "type": "STRING",
                        "description": "Description of the problem or error signature."
                    },
                    "solution": {
                        "type": "STRING",
                        "description": "Resolution steps, fix description, or patch details."
                    }
                },
                "required": ["problem", "solution"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let problem = match args.get("problem").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'problem' parameter is required.".to_string(),
        };
        let solution = match args.get("solution").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return "Error: 'solution' parameter is required.".to_string(),
        };

        let mut store = MemoryStore::load();
        match store.add_fix(problem, solution) {
            Ok(()) => "Successfully recorded past fix.".to_string(),
            Err(e) => format!("Failed to save memory: {}", e),
        }
    }
}

pub struct RecordFileRelationTool;

#[async_trait]
impl AgentTool for RecordFileRelationTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "record_file_relation".to_string(),
            description: "Record a structural relationship between files and symbols (e.g. function calls, trait implementation, dependencies) in memory.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "file_path": {
                        "type": "STRING",
                        "description": "Source file path."
                    },
                    "related_symbol": {
                        "type": "STRING",
                        "description": "Symbol name (function, struct, enum, etc.)."
                    },
                    "relation_type": {
                        "type": "STRING",
                        "description": "Relation type (e.g. 'calls', 'implements', 'depends_on')."
                    },
                    "target_file": {
                        "type": "STRING",
                        "description": "Target file path."
                    },
                    "notes": {
                        "type": "STRING",
                        "description": "Additional context or notes about the relationship."
                    }
                },
                "required": ["file_path", "related_symbol", "relation_type", "target_file", "notes"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
            Some(f) => f,
            None => return "Error: 'file_path' parameter is required.".to_string(),
        };
        let related_symbol = match args.get("related_symbol").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return "Error: 'related_symbol' parameter is required.".to_string(),
        };
        let relation_type = match args.get("relation_type").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: 'relation_type' parameter is required.".to_string(),
        };
        let target_file = match args.get("target_file").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return "Error: 'target_file' parameter is required.".to_string(),
        };
        let notes = match args.get("notes").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return "Error: 'notes' parameter is required.".to_string(),
        };

        let mut store = MemoryStore::load();
        match store.add_file_relation(file_path, related_symbol, relation_type, target_file, notes)
        {
            Ok(()) => "Successfully recorded file and symbol relationship.".to_string(),
            Err(e) => format!("Failed to save memory: {}", e),
        }
    }
}
