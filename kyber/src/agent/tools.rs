use std::process::Command as ShellCommand;
use std::sync::Mutex;
use headless_chrome::{Browser, LaunchOptions, Tab};

pub struct Tools {
    browser: Mutex<Option<Browser>>,
}

impl Tools {
    pub fn new() -> Self {
        Tools { browser: Mutex::new(None) }
    }

    /// Lazy-init browser — launched once, reused across operations.
    fn get_browser(&self) -> Result<std::sync::MutexGuard<'_, Option<Browser>>, String> {
        self.browser.lock().map_err(|e| format!("浏览器锁失败: {}", e))
    }

    fn ensure_browser(&self) -> Result<(), String> {
        let mut guard = self.get_browser()?;
        if guard.is_none() {
            let launch_opts = LaunchOptions {
                headless: true,
                sandbox: false, // needed on some systems
                window_size: Some((1280, 720)),
                ..LaunchOptions::default()
            };
            let browser = Browser::new(launch_opts)
                .map_err(|e| format!("启动浏览器失败: {}。请确保 Chrome/Chromium 已安装。", e))?;
            *guard = Some(browser);
        }
        Ok(())
    }

    fn with_tab<F, R>(&self, f: F) -> Result<R, String>
    where F: FnOnce(&Tab) -> Result<R, String>
    {
        self.ensure_browser()?;
        let guard = self.get_browser()?;
        let browser = guard.as_ref().ok_or("浏览器未初始化")?;
        let tab = browser.new_tab().map_err(|e| format!("创建标签页失败: {}", e))?;
        f(&tab)
    }

    // ── File tools ──

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

    // ── Browser tools ──

    /// Navigate to URL and return page text content.
    pub fn browser_navigate(&self, url: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            tab.navigate_to(url).map_err(|e| format!("导航失败: {}", e))?;
            tab.wait_until_navigated().map_err(|e| format!("等待页面加载失败: {}", e))?;

            // Get visible text via JS
            let text = tab.evaluate("document.body.innerText", false)
                .map_err(|e| format!("读取页面文本失败: {}", e))?;
            let text_str = text.value
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "(无文本内容)".into());

            Ok(text_str)
        })
    }

    /// Take a screenshot and save to file.
    pub fn browser_screenshot(&self, path: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            let data = tab.capture_screenshot(
                headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                None,
                None,
                true,
            ).map_err(|e| format!("截图失败: {}", e))?;

            std::fs::write(path, &data).map_err(|e| format!("保存截图失败: {}", e))?;
            Ok(format!("截图已保存至 {}", path))
        })
    }

    /// Click an element by CSS selector.
    pub fn browser_click(&self, selector: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            tab.wait_for_element(selector)
                .map_err(|e| format!("未找到元素 '{}': {}", selector, e))?;
            let element = tab.find_element(selector)
                .map_err(|e| format!("查找元素失败: {}", e))?;
            element.click().map_err(|e| format!("点击失败: {}", e))?;
            Ok(format!("已点击 '{}'", selector))
        })
    }

    /// Type text into an input element.
    pub fn browser_type(&self, selector: &str, text: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            tab.wait_for_element(selector)
                .map_err(|e| format!("未找到元素 '{}': {}", selector, e))?;
            let element = tab.find_element(selector)
                .map_err(|e| format!("查找元素失败: {}", e))?;
            element.click().map_err(|e| format!("聚焦元素失败: {}", e))?;
            element.type_into(text).map_err(|e| format!("输入失败: {}", e))?;
            Ok(format!("已在 '{}' 输入文本", selector))
        })
    }

    /// Get text content of an element by CSS selector.
    pub fn browser_get_text(&self, selector: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            let js = format!(
                "document.querySelector('{}') ? document.querySelector('{}').innerText : '(未找到)'",
                selector.replace('\'', "\\'"),
                selector.replace('\'', "\\'"),
            );
            let result = tab.evaluate(&js, false)
                .map_err(|e| format!("执行 JS 失败: {}", e))?;
            let text = result.value
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "(无内容)".into());
            Ok(text)
        })
    }

    /// Execute arbitrary JavaScript in the page context.
    pub fn browser_evaluate(&self, js: &str) -> Result<String, String> {
        self.with_tab(|tab| {
            let result = tab.evaluate(js, false)
                .map_err(|e| format!("执行 JS 失败: {}", e))?;
            let text = format!("{:?}", result.value);
            Ok(text)
        })
    }

    /// Read the current page URL.
    pub fn browser_current_url(&self) -> Result<String, String> {
        self.with_tab(|tab| {
            let url = tab.get_url();
            Ok(url)
        })
    }

    // ── Action dispatcher ──

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

            // Browser actions
            "browser" | "navigate" => {
                let url = action.params.get("url").map(String::as_str)
                    .ok_or("缺少 url 参数")?;
                self.browser_navigate(url)
            }
            "screenshot" => {
                let path = action.params.get("path").map(String::as_str)
                    .unwrap_or("screenshot.png");
                self.browser_screenshot(path)
            }
            "click" => {
                let selector = action.params.get("selector").map(String::as_str)
                    .ok_or("缺少 selector 参数")?;
                self.browser_click(selector)
            }
            "type" => {
                let selector = action.params.get("selector").map(String::as_str)
                    .ok_or("缺少 selector 参数")?;
                let text = action.params.get("text").map(String::as_str)
                    .unwrap_or("");
                self.browser_type(selector, text)
            }
            "get_text" => {
                let selector = action.params.get("selector").map(String::as_str)
                    .ok_or("缺少 selector 参数")?;
                self.browser_get_text(selector)
            }
            "evaluate" => {
                let js = action.params.get("js").map(String::as_str)
                    .ok_or("缺少 js 参数")?;
                self.browser_evaluate(js)
            }

            "think" | "respond" => {
                Ok(action.description.clone())
            }
            _ => Err(format!("未知操作: {}", action.kind)),
        }
    }
}
