use colored::*;
use similar::{ChangeTag, TextDiff};
use std::io;
use std::path::{Path, PathBuf};
use tokio::process::Command;

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
                            format!(
                                "Access denied: '{:?}' escapes workspace boundary",
                                user_path
                            ),
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

            if !resolved.starts_with(&canonical_workspace) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "Access denied: '{:?}' escapes workspace boundary",
                        user_path
                    ),
                ));
            }
        }

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
                format!(
                    "Access denied: '{:?}' escapes workspace boundary",
                    user_path
                ),
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

        self.create_git_checkpoint(&format!("rune: pre-write checkpoint for {:?}", path))
            .await;

        let old_content = tokio::fs::read_to_string(&safe_path)
            .await
            .unwrap_or_default();

        println!(
            "\n{}: {}",
            "PREVIEW CHANGES FOR".bold().cyan(),
            path.display().to_string().yellow()
        );
        let diff = TextDiff::from_lines(old_content.as_str(), content);
        for change in diff.iter_all_changes() {
            let (sign, line_str) = match change.tag() {
                ChangeTag::Delete => ("- ", format!("{}", change).red()),
                ChangeTag::Insert => ("+ ", format!("{}", change).green()),
                ChangeTag::Equal => ("  ", format!("{}", change).normal()),
            };
            print!("{}{}", sign.dimmed(), line_str);
        }
        println!();

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

    pub async fn preview_patch(
        &self,
        path: &Path,
        search: &str,
        replace: &str,
    ) -> Result<(PathBuf, String), std::io::Error> {
        let safe_path = self.sanitize_path(path)?;

        self.create_git_checkpoint(&format!("rune: pre-patch checkpoint for {:?}", path))
            .await;

        let old_content = match tokio::fs::read_to_string(&safe_path).await {
            Ok(c) => c,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Failed to read file for patching {:?}: {}", safe_path, e),
                ));
            }
        };

        let count = old_content.matches(search).count();
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Search block not found in file {:?}", path),
            ));
        }
        if count > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Search block matches {} times in file {:?}. Must be unique.",
                    count, path
                ),
            ));
        }

        let new_content = old_content.replacen(search, replace, 1);

        println!(
            "\n{}: {}",
            "PREVIEW PATCH FOR".bold().cyan(),
            path.display().to_string().yellow()
        );
        let diff = TextDiff::from_lines(old_content.as_str(), new_content.as_str());
        for change in diff.iter_all_changes() {
            let (sign, line_str) = match change.tag() {
                ChangeTag::Delete => ("- ", format!("{}", change).red()),
                ChangeTag::Insert => ("+ ", format!("{}", change).green()),
                ChangeTag::Equal => ("  ", format!("{}", change).normal()),
            };
            print!("{}{}", sign.dimmed(), line_str);
        }
        println!();

        Ok((safe_path, new_content))
    }

    pub async fn apply_patch(
        &self,
        safe_path: &Path,
        new_content: &str,
        search_len: usize,
        replace_len: usize,
    ) -> Result<String, std::io::Error> {
        tokio::fs::write(safe_path, new_content).await?;

        Ok(format!(
            "Successfully patched file {:?} (replaced {} bytes with {} bytes)",
            safe_path, search_len, replace_len
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
                    Ok(format!(
                        "cix search output:\n{}\nSTDERR:\n{}",
                        stdout, stderr
                    ))
                } else if !output.status.success() {
                    Ok(format!(
                        "cix search error (Exit Code {}):\n{}",
                        output.status, stderr
                    ))
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
        results.push_str(
            "[Note: 'cix' binary not found. Falling back to recursive workspace search]\n\n",
        );

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
                                results.push_str(&format!(
                                    "{}:{}: {}\n",
                                    rel_path.display(),
                                    line_num + 1,
                                    line
                                ));
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

    pub async fn execute_batch(&self, commands: &[String]) -> Result<String, std::io::Error> {
        self.create_git_checkpoint("rune: pre-batch execution checkpoint")
            .await;
        let mut batch_output = String::new();

        for (i, cmd) in commands.iter().enumerate() {
            batch_output.push_str(&format!(
                "=== [Command {}/{}: {}] ===\n",
                i + 1,
                commands.len(),
                cmd
            ));
            match self.execute_commands(cmd).await {
                Ok(result) => {
                    batch_output.push_str(&result);
                    batch_output.push_str("\n\n");

                    if result.contains("Exit Code: 1")
                        || result.contains("Exit Code: exit status: 1")
                        || result.contains("Exit Code: 101")
                    {
                        batch_output
                            .push_str("[Batch execution halted due to non-zero exit code]\n");
                        break;
                    }
                }
                Err(e) => {
                    batch_output.push_str(&format!("Error executing command: {}\n\n", e));
                    break;
                }
            }
        }

        Ok(batch_output)
    }

    pub async fn undo_git_checkpoint(&self) -> Result<String, std::io::Error> {
        let canonical_workspace = dunce::canonicalize(&self.workspace_root)?;

        #[cfg(target_os = "windows")]
        let status = Command::new("cmd")
            .current_dir(&canonical_workspace)
            .args(["/C", "git stash pop"])
            .status()
            .await?;

        #[cfg(not(target_os = "windows"))]
        let status = Command::new("sh")
            .current_dir(&canonical_workspace)
            .args(["-c", "git stash pop"])
            .status()
            .await?;

        if status.success() {
            Ok(
                "Successfully popped git stash checkpoint (reverted last agent file mutation)."
                    .to_string(),
            )
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to pop git stash (no stash checkpoint found or git conflict).",
            ))
        }
    }

    pub async fn git_status(&self) -> Result<String, std::io::Error> {
        self.execute_commands("git status").await
    }

    pub async fn git_diff(&self, path: Option<&str>) -> Result<String, std::io::Error> {
        let cmd = match path {
            Some(p) if !p.is_empty() => format!("git diff {}", p),
            _ => "git diff".to_string(),
        };
        self.execute_commands(&cmd).await
    }

    pub async fn git_commit(&self, message: &str) -> Result<String, std::io::Error> {
        let canonical_workspace = dunce::canonicalize(&self.workspace_root)?;

        let add_output = Command::new("git")
            .current_dir(&canonical_workspace)
            .args(["add", "-A"])
            .output()
            .await?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Ok(format!("git add failed:\n{}", stderr));
        }

        let commit_output = Command::new("git")
            .current_dir(&canonical_workspace)
            .args(["commit", "-m", message])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        let stderr = String::from_utf8_lossy(&commit_output.stderr);

        Ok(format!(
            "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            commit_output.status, stdout, stderr
        ))
    }

    async fn create_git_checkpoint(&self, message: &str) {
        let canonical_workspace = match dunce::canonicalize(&self.workspace_root) {
            Ok(w) => w,
            Err(_) => return,
        };

        let is_git = Command::new("git")
            .current_dir(&canonical_workspace)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !is_git {
            return;
        }

        let _ = Command::new("git")
            .current_dir(&canonical_workspace)
            .args(["add", "-A"])
            .output()
            .await;

        let _ = Command::new("git")
            .current_dir(&canonical_workspace)
            .args(["stash", "push", "-m", message])
            .output()
            .await;
    }

    pub async fn preview_write(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let safe_path = self.sanitize_path(path)?;
        self.create_git_checkpoint(&format!("rune: pre-write checkpoint for {:?}", path))
            .await;

        let old_content = tokio::fs::read_to_string(&safe_path)
            .await
            .unwrap_or_default();

        println!(
            "\n{}: {}",
            "PREVIEW CHANGES FOR".bold().cyan(),
            path.display().to_string().yellow()
        );
        let diff = TextDiff::from_lines(old_content.as_str(), content);
        for change in diff.iter_all_changes() {
            let (sign, line_str) = match change.tag() {
                ChangeTag::Delete => ("- ", format!("{}", change).red()),
                ChangeTag::Insert => ("+ ", format!("{}", change).green()),
                ChangeTag::Equal => ("  ", format!("{}", change).normal()),
            };
            print!("{}{}", sign.dimmed(), line_str);
        }
        println!();
        let _ = std::io::Write::flush(&mut std::io::stdout());

        Ok(safe_path)
    }

    pub async fn apply_write(
        &self,
        safe_path: &Path,
        content: &str,
    ) -> Result<String, std::io::Error> {
        if let Some(parent) = safe_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(safe_path, content).await?;
        Ok(format!(
            "Successfully wrote {} bytes to {:?}",
            content.len(),
            safe_path
        ))
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

    #[tokio::test]
    async fn test_patch_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello world!\nThis is a test file.\nGoodbye.\n")
            .await
            .unwrap();

        let executor = ToolExecutor::new(dir.path().to_path_buf());
        let (safe_path, new_content) = executor
            .preview_patch(
                Path::new("test.txt"),
                "This is a test file.",
                "This is a patched file.",
            )
            .await
            .unwrap();
        let res = executor
            .apply_patch(
                &safe_path,
                &new_content,
                "This is a test file.".len(),
                "This is a patched file.".len(),
            )
            .await;
        assert!(res.is_ok());

        let new_content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(new_content.contains("This is a patched file."));
        assert!(!new_content.contains("This is a test file."));
    }
}
