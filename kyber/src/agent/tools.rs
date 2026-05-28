use std::process::Command as ShellCommand;

pub struct Tools;

impl Tools {
    pub fn new() -> Self { Tools }

    pub fn read_file(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("读文件失败: {}", e))
    }

    pub fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }
        std::fs::write(path, content).map_err(|e| format!("写文件失败: {}", e))
    }

    pub fn list_files(&self, path: &str) -> Result<Vec<String>, String> {
        let entries = std::fs::read_dir(path).map_err(|e| format!("读目录失败: {}", e))?;
        let mut files = Vec::new();
        for entry in entries {
            if let Ok(e) = entry {
                if let Ok(name) = e.file_name().into_string() {
                    files.push(name);
                }
            }
        }
        Ok(files)
    }

    pub fn execute_command(&self, command: &str) -> Result<String, String> {
        let output = if cfg!(target_os = "windows") {
            ShellCommand::new("cmd").args(["/C", command]).output()
        } else {
            ShellCommand::new("sh").args(["-c", command]).output()
        }.map_err(|e| format!("执行命令失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(format!("命令退出码 {}: {}", output.status, stderr))
        }
    }

    pub async fn browser_get(&self, url: &str) -> Result<String, String> {
        // Simple HTTP GET via reqwest (no browser needed for basic web fetching)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP 客户端创建失败: {}", e))?;

        let response = client.get(url)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        let text = response.text().await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        Ok(text)
    }

    pub fn execute_action(&self, action: &super::controller::Action) -> Result<String, String> {
        match action.kind.as_str() {
            "read" => {
                let path = action.params.get("path").map(String::as_str).unwrap_or(".");
                self.read_file(path)
            }
            "write" => {
                let path = action.params.get("path").map(String::as_str).unwrap_or("output.txt");
                let content = action.params.get("content").map(String::as_str).unwrap_or("");
                self.write_file(path, content)?;
                Ok(format!("已写入 {}", path))
            }
            "execute" => {
                let cmd = action.params.get("command").map(String::as_str).unwrap_or("echo ok");
                self.execute_command(cmd)
            }
            "think" | "respond" => {
                Ok(action.description.clone())
            }
            _ => Err(format!("未知操作: {}", action.kind)),
        }
    }
}
