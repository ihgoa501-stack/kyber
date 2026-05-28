use super::observer::Observation;
use super::llm::Backend;

#[derive(Debug, Clone)]
pub struct Action {
    pub kind: String,
    pub description: String,
    pub params: std::collections::HashMap<String, String>,
}

impl Action {
    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn needs_confirm(&self) -> bool {
        matches!(self.kind.as_str(), "delete" | "write" | "execute" | "git_push")
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind, self.description)
    }
}

pub struct Controller {
    pub max_iterations: u32,
    pub steps_taken: u32,
    pub done: bool,
    pub task: String,
    pub backend: Backend,
}

impl Controller {
    pub fn new(max_iterations: u32, task: String, backend: Backend) -> Self {
        Controller { max_iterations, steps_taken: 0, done: false, task, backend }
    }

    /// Decide next action using LLM.
    pub async fn decide(&mut self, obs: &Observation) -> Action {
        self.steps_taken += 1;

        if self.steps_taken >= self.max_iterations {
            self.done = true;
            return Action {
                kind: "respond".into(),
                description: format!("我已执行 {} 步，任务: {}", self.steps_taken, self.task),
                params: std::collections::HashMap::new(),
            };
        }

        // Check done condition
        if obs.confidence > 0.95 && obs.issues.is_empty() {
            // If confidence is very high and no issues, ask LLM if we're done
            let done_prompt = format!(
                "任务: {}\n当前状态: {}\n问题: {:?}\n这个任务完成了吗？只回答 DONE 或 CONTINUE",
                self.task, obs.summary, obs.issues
            );
            if let Ok(response) = super::llm::call(&self.backend, "你决定任务是否完成。只输出 DONE 或 CONTINUE。", &done_prompt).await {
                if response.trim().contains("DONE") {
                    self.done = true;
                    return Action {
                        kind: "respond".into(),
                        description: format!("任务完成: {}", obs.summary),
                        params: std::collections::HashMap::new(),
                    };
                }
            }
        }

        // Generate next action via LLM
        let sys_prompt = r#"你是 Kyber Agent 的决策控制器。输出下一步行动。**必须包含 params 字段**。JSON 格式:
{
  "kind": "read|write|execute|navigate|click|type|screenshot|get_text|evaluate|think|respond",
  "description": "做什么",
  "params": {}
}
文件操作:
- read: 读文件 (params: path)
- write: 写文件 (params: path, content)
- execute: 终端命令 (params: command)
浏览器操作:
- navigate: 打开网页 (params: url)
- click: 点击元素 (params: selector)
- type: 输入文本 (params: selector, text)
- screenshot: 截图 (params: path)
- get_text: 读元素文本 (params: selector)
- evaluate: 执行JS (params: js)
其他:
- think: 内部推理
- respond: 回复用户 (params: message)"#;

        let action_prompt = format!(
            "任务: {}\n步骤: {}/{}\n状态: {}\n问题: {:?}\n\n输出下一个 JSON 行动。",
            self.task, self.steps_taken, self.max_iterations, obs.summary, obs.issues
        );

        match super::llm::call(&self.backend, sys_prompt, &action_prompt).await {
            Ok(text) => {
                if let Some(json_str) = extract_json(&text) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let kind = parsed.get("kind").and_then(|k| k.as_str()).unwrap_or("think").to_string();
                        let description = parsed.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
                        let mut params: std::collections::HashMap<String, String> = parsed.get("params")
                            .and_then(|p| p.as_object())
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                            .unwrap_or_default();

                        // Heuristic: fill missing params from description
                        fill_missing_params(&kind, &description, &mut params);

                        return Action { kind, description, params };
                    }
                }
                Action { kind: "think".into(), description: text, params: std::collections::HashMap::new() }
            }
            Err(_) => {
                Action { kind: "respond".into(), description: "LLM 不可用，请设置 ANTHROPIC_API_KEY".into(), params: std::collections::HashMap::new() }
            }
        }
    }

    pub fn is_done(&self) -> bool { self.done }
    pub fn handle_failure(&mut self, _action: &Action) {
        self.steps_taken = self.steps_taken.saturating_sub(1);
    }
}

/// Fill missing params heuristically from the description.
/// DeepSeek and other models sometimes omit params.command even when kind=execute.
fn fill_missing_params(kind: &str, desc: &str, params: &mut std::collections::HashMap<String, String>) {
    if !params.is_empty() { return; } // params already provided, trust the LLM

    match kind {
        "execute" => {
            if !params.contains_key("command") {
                let cmd = if desc.contains("ls")
                    || desc.contains("列出") || desc.contains("list")
                    || desc.contains("目录") { "ls -la".into() }
                else if desc.contains("pwd") || desc.contains("当前")
                    || desc.contains("工作目录") { "pwd".into() }
                else if desc.contains("cat") || desc.contains("读取")
                    || desc.contains("read") || desc.contains("内容") { "cat README.md".into() }
                else if desc.contains("git") { "git status".into() }
                else if desc.contains("find") || desc.contains("搜索")
                    || desc.contains("查找") { "find . -name '*.rs' -maxdepth 3".into() }
                else { "ls -la".into() };
                params.insert("command".into(), cmd);
            }
        }
        "read" => {
            if !params.contains_key("path") {
                let path = if desc.contains("README") { "README.md".into() }
                else if desc.contains("Cargo") { "Cargo.toml".into() }
                else { ".".into() };
                params.insert("path".into(), path);
            }
        }
        "navigate" => {
            if !params.contains_key("url") {
                params.insert("url".into(), "https://github.com".into());
            }
        }
        _ => {}
    }
}

fn extract_json(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Some(text[start..=end].to_string());
        }
    }
    None
}
