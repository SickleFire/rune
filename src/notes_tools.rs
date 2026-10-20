use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn notes_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join(".rune").join("notes");
        let _ = fs::create_dir_all(&dir);
        return dir;
    }
    PathBuf::from(".rune/notes")
}

pub struct ListRuneNotesTool;

#[async_trait]
impl AgentTool for ListRuneNotesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "list_rune_notes".to_string(),
            description: "List all available rune-notes (knowledge nodes) in .rune/notes/".to_string(),
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

    async fn execute(&self, _args: serde_json::Value) -> String {
        let dir = notes_dir();
        if !dir.exists() {
            return "No rune-notes directory found (.rune/notes/ is empty).".to_string();
        }

        let mut notes = Vec::new();
        let mut stack = vec![dir.clone()];
        while let Some(path) = stack.pop() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().map_or(false, |ext| ext == "md" || ext == "markdown") {
                        if let Ok(rel) = p.strip_prefix(&dir) {
                            notes.push(rel.to_string_lossy().to_string());
                        } else {
                            notes.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        notes.sort();
        if notes.is_empty() {
            return "No rune-notes (.md files) found in .rune/notes/.".to_string();
        }

        format!("Available rune-notes:\n- {}", notes.join("\n- "))
    }
}

pub struct ReadRuneNoteTool;

#[async_trait]
impl AgentTool for ReadRuneNoteTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "read_rune_note".to_string(),
            description: "Read the content of a specific rune-note (knowledge node) by its relative path or ID (e.g. 'overview.md' or 'src/api.rs.md').".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "note_path": {
                        "type": "STRING",
                        "description": "The path or name of the note inside .rune/notes/ (e.g., 'overview.md' or 'concepts/godot.md')."
                    }
                },
                "required": ["note_path"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let note_path = match args.get("note_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'note_path' parameter is required.".to_string(),
        };

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        // Security check against directory traversal
        if let Ok(canonical_dir) = fs::canonicalize(&dir) {
            if let Ok(canonical_target) = fs::canonicalize(&full_path) {
                if !canonical_target.starts_with(&canonical_dir) {
                    return "Error: Access denied (path outside .rune/notes/).".to_string();
                }
            }
        }

        if !full_path.exists() {
            return format!("Error: Rune-note '{}' not found in .rune/notes/.", note_path);
        }

        match fs::read_to_string(&full_path) {
            Ok(content) => format!("--- Rune Note: {} ---\n\n{}", note_path, content),
            Err(e) => format!("Error reading rune-note '{}': {}", note_path, e),
        }
    }
}

pub struct UpsertRuneNoteTool;

#[async_trait]
impl AgentTool for UpsertRuneNoteTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "upsert_rune_note".to_string(),
            description: "Create or update a rune-note (knowledge node) in .rune/notes/ with Markdown content.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "note_path": {
                        "type": "STRING",
                        "description": "The path or name of the note (e.g., 'overview.md' or 'src/api.rs.md')."
                    },
                    "content": {
                        "type": "STRING",
                        "description": "The full Markdown content of the note (including frontmatter if desired)."
                    }
                },
                "required": ["note_path", "content"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: serde_json::Value) -> String {
        let note_path = match args.get("note_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'note_path' parameter is required.".to_string(),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        let dir = notes_dir();
        let full_path = dir.join(note_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(canonical_dir) = fs::canonicalize(&dir) {
            if let Some(parent) = full_path.parent() {
                if let Ok(canonical_parent) = fs::canonicalize(parent) {
                    if !canonical_parent.starts_with(&canonical_dir) {
                        return "Error: Access denied (path outside .rune/notes/).".to_string();
                    }
                }
            }
        }

        match fs::write(&full_path, content) {
            Ok(()) => format!("Successfully saved rune-note '{}'.", note_path),
            Err(e) => format!("Error saving rune-note '{}': {}", note_path, e),
        }
    }
}
