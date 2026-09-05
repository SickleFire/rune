use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct ToolExecutor {
    workspace_root: PathBuf,
}

impl ToolExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn read_file(&self, path: &Path) -> Result<String, std::io::Error> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }

    pub async fn write_file(&self, path: &Path, content: &str) -> Result<String, std::io::Error> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, content).await?;

        Ok(format!(
            "Successfully wrote {} bytes to {:?}",
            content.len(),
            path
        ))
    }

    pub async fn list_files(&self, path: &Path) -> Result<String, std::io::Error> {
        let mut entries = tokio::fs::read_dir(path).await?;
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

    pub async fn execute_commands(&self, cmd: &str) -> Result<String, std::io::Error> {
        let output = Command::new("cmd").args(["/C", cmd]).output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = format!(
            "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status, stdout, stderr
        );

        Ok(result)
    }
}
