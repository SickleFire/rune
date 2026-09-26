use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct MemoryStore {
    #[serde(default)]
    pub preferences: HashMap<String, String>,
    #[serde(default)]
    pub past_fixes: Vec<PastFix>,
    #[serde(default)]
    pub file_relations: Vec<FileRelation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PastFix {
    pub problem: String,
    pub solution: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileRelation {
    pub file_path: String,
    pub related_symbol: String,
    pub relation_type: String, // e.g., "calls", "implements", "depends_on"
    pub target_file: String,
    pub notes: String,
}

impl MemoryStore {
    pub fn memory_file_path() -> PathBuf {
        if let Ok(path) = std::env::var("RUNE_MEMORY_PATH") {
            return PathBuf::from(path);
        }
        if let Ok(cwd) = std::env::current_dir() {
            let local_dir = cwd.join(".rune");
            let _ = fs::create_dir_all(&local_dir);
            return local_dir.join("memory.json");
        }

        if let Some(home) = dirs_or_home() {
            let config_dir = home.join(".config").join("rune");
            if fs::create_dir_all(&config_dir).is_ok() {
                return config_dir.join("memory.json");
            }
        }

        PathBuf::from("rune_memory.json")
    }

    pub fn load() -> Self {
        let path = Self::memory_file_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str(&data) {
                    return store;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::memory_file_path();
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn set_preference(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), String> {
        let mut latest = Self::load();
        latest.preferences.insert(key.into(), value.into());
        latest.save()?;
        *self = latest;
        Ok(())
    }

    pub fn get_preference(&self, key: &str) -> Option<&String> {
        self.preferences.get(key)
    }

    pub fn add_fix(
        &mut self,
        problem: impl Into<String>,
        solution: impl Into<String>,
    ) -> Result<(), String> {
        let mut latest = Self::load();
        latest.past_fixes.push(PastFix {
            problem: problem.into(),
            solution: solution.into(),
        });
        latest.save()?;
        *self = latest;
        Ok(())
    }

    pub fn search_fixes(&self, query: &str) -> Vec<&PastFix> {
        let q = query.to_lowercase();
        self.past_fixes
            .iter()
            .filter(|f| {
                f.problem.to_lowercase().contains(&q) || f.solution.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn add_file_relation(
        &mut self,
        file_path: impl Into<String>,
        related_symbol: impl Into<String>,
        relation_type: impl Into<String>,
        target_file: impl Into<String>,
        notes: impl Into<String>,
    ) -> Result<(), String> {
        let mut latest = Self::load();
        latest.file_relations.push(FileRelation {
            file_path: file_path.into(),
            related_symbol: related_symbol.into(),
            relation_type: relation_type.into(),
            target_file: target_file.into(),
            notes: notes.into(),
        });
        latest.save()?;
        *self = latest;
        Ok(())
    }

    pub fn query_relations(&self, query: &str) -> Vec<&FileRelation> {
        let q = query.to_lowercase();
        self.file_relations
            .iter()
            .filter(|r| {
                r.file_path.to_lowercase().contains(&q)
                    || r.related_symbol.to_lowercase().contains(&q)
                    || r.target_file.to_lowercase().contains(&q)
                    || r.notes.to_lowercase().contains(&q)
            })
            .collect()
    }
}

fn dirs_or_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
