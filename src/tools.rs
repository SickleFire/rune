use std::path::{Path, PathBuf};
use tokio::process::Command;
use std::io;

pub struct ToolExecutor {
    workspace_root: PathBuf,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn sanitize_path(&self, user_path: &Path) -> io::Result<PathBuf> {
        let canonical_workspace = dunce::canonicalize(&self.workspace_root)?;
        
        if user_path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Access denied: absolute path '{:?}' not allowed", user_path),
            ));
        }

        // Evaluate path structure through Path::components()
        let mut resolved = canonical_workspace.clone();
        for component in user_path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    resolved.push(name);
                }
                std::path::Component::CurDir => {
                    // '.' - do nothing
                }
                std::path::Component::ParentDir => {
                    // '..' - pop from resolved
                    if !resolved.pop() {
                        return Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            format!("Access denied: '{:?}' escapes workspace boundary", user_path),
                        ));
                    }
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("Invalid path component in '{:?}'", user_path),
                    ));
                }
            }

            // Ensure we haven't escaped the workspace boundary during component accumulation
            if !resolved.starts_with(&canonical_workspace) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Access denied: '{:?}' escapes workspace boundary", user_path),
                ));
            }
        }

        // Handle non-existent paths by walking up ancestors of the resolved path
        let mut ancestor = resolved.as_path();
        let mut tail_components = Vec::new();

        while !ancestor.exists() {
            if let Some(parent) = ancestor.parent() {
                if let Some(name) = ancestor.file_name() {
                    tail_components.push(name);
                }
                ancestor = parent;
            } else {
                break;
            }
        }

        tail_components.reverse();

        let canonical_ancestor = dunce::canonicalize(ancestor)?;

        let mut final_target = canonical_ancestor;
        for component in tail_components {
            final_target.push(component);
        }

        if !final_target.starts_with(&canonical_workspace) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Access denied: '{:?}' escapes workspace boundary", user_path),
            ));
        }

        Ok(final_target)
    }

    pub async fn read_file(&self, path: &Path) -> Result<String, std::io::Error> {
        let safe_path = self.sanitize_path(path)?;
        let content = tokio::fs::read_to_string(safe_path).await?;
        Ok(content)
    }

    pub async fn write_file(&self, path: &Path, content: &str) -> Result<String, std::io::Error> {
        let safe_path = self.sanitize_path(path)?;
        if let Some(parent) = safe_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(safe_path.clone(), content).await?;

        Ok(format!(
            "Successfully wrote {} bytes to {:?}",
            content.len(),
            safe_path
        ))
    }

    pub async fn list_files(&self, path: &Path) -> Result<String, std::io::Error> {
        let safe_path = self.sanitize_path(path)?;
        let mut entries = tokio::fs::read_dir(safe_path).await?;
        let mut output = String::new();

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let string = &file_name.to_string_lossy();
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                output.push_str(&format!("[DIR]: {}\n", string));
            } else {
                output.push_str(&format!("[FILE]: {}\n", string));
            }
        }
        Ok(output)
    }

    pub async fn search_code(&self, query: &str) -> Result<String, std::io::Error> {
        let canonical_workspace = dunce::canonicalize(&self.workspace_root)?;
        
        let cix_result = Command::new("cix")
            .current_dir(&canonical_workspace)
            .arg(query)
            .arg(&canonical_workspace)
            .output()
            .await;

        match cix_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !output.status.success() && !stdout.trim().is_empty() {
                    Ok(format!("cix search output:\n{}\nSTDERR:\n{}", stdout, stderr))
                } else if !output.status.success() {
                    Ok(format!("cix search error (Exit Code {}):\n{}", output.status, stderr))
                } else {
                    Ok(stdout.to_string())
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Self::fallback_search(&canonical_workspace, query).await
            }
            Err(e) => Err(e),
        }
    }

    async fn fallback_search(workspace_root: &Path, query: &str) -> Result<String, std::io::Error> {
        let mut results = String::new();
        results.push_str("[Note: 'cix' binary not found. Falling back to recursive workspace search]\n\n");

        let mut stack = vec![workspace_root.to_path_buf()];
        let query_lower = query.to_lowercase();

        let ignore_dirs = [".git", "target", ".cargo", "node_modules"];

        while let Some(dir) = stack.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if path.is_dir() {
                    if ignore_dirs.contains(&file_name_str.as_ref()) {
                        continue;
                    }
                    stack.push(path);
                } else if path.is_file() {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        for (line_num, line) in content.lines().enumerate() {
                            if line.to_lowercase().contains(&query_lower) {
                                let rel_path = path.strip_prefix(workspace_root).unwrap_or(&path);
                                results.push_str(&format!("{}:{}: {}\n", rel_path.display(), line_num + 1, line));
                            }
                        }
                    }
                }
            }
        }

        if results.lines().count() <= 1 {
            results.push_str(&format!("No matches found for query '{}'.\n", query));
        }

        Ok(results)
    }

    pub async fn execute_commands(&self, cmd: &str) -> Result<String, std::io::Error> {
        let canonical_workspace = dunce::canonicalize(&self.workspace_root)?;

        #[cfg(target_os = "windows")]
        let output = Command::new("cmd")
            .current_dir(canonical_workspace)
            .args(["/C", cmd])
            .output()
            .await?;

        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh")
            .current_dir(canonical_workspace)
            .args(["-c", cmd])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = format!(
            "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status, stdout, stderr
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_path_normal() {
        let dir = tempdir().unwrap();
        let executor = ToolExecutor::new(dir.path().to_path_buf());
        let safe = executor.sanitize_path(Path::new("src/main.rs")).unwrap();
        assert!(safe.starts_with(dunce::canonicalize(dir.path()).unwrap()));
    }

    #[test]
    fn test_sanitize_path_relative_traversal_escape() {
        let dir = tempdir().unwrap();
        let executor = ToolExecutor::new(dir.path().to_path_buf());
        let res = executor.sanitize_path(Path::new("non_existent_dir/../../etc/passwd"));
        assert!(res.is_err());
    }

    #[test]
    fn test_sanitize_path_dot_dot_inside() {
        let dir = tempdir().unwrap();
        let executor = ToolExecutor::new(dir.path().to_path_buf());
        let safe = executor.sanitize_path(Path::new("subdir/../file.txt"));
        assert!(safe.is_ok());
    }
}
