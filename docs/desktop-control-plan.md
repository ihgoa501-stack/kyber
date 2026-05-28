# 桌面操控计划

## 控制目标

让 Kyber 能够操作桌面应用——点击窗口、输入文本、读取界面内容。

## 架构

桌面操控本质上就是一层 L1 工具。不和浏览器、终端耦合，在 `tools.rs` 里加一个 `DesktopTool`，和现有的 `Browser` 同级。

```
tools.rs
├── Filesystem     (已有)
├── Terminal       (已有)
├── Browser        (刚完成, headless_chrome)
└── Desktop        (新增)
```

## 平台方案

| 平台 | 工具 | 能力 |
|---|---|---|
| macOS | `osascript` + Accessibility API | 窗口焦点、键鼠模拟、UI 元素读取 |
| Linux | `xdotool` + `AT-SPI` | 同上 |
| Windows | `uiautomation` (PowerShell) | 同上 |

首版：仅 macOS（`osascript` + AppleScript），因为 `headless_chrome` 同样依赖 macOS 生态。

## macOS 实现

不需要额外 Rust 依赖。直接 `execute_command("osascript -e '...'")`：

| 操作 | AppleScript | Kyber action |
|---|---|---|
| 获取前台应用 | `tell app "System Events" to get name of first process whose frontmost is true` | `desktop/get_active_app` |
| 切换窗口 | `tell app "System Events" to set frontmost of process "Safari" to true` | `desktop/focus` |
| 按键 | `tell app "System Events" to keystroke "hello"` | `desktop/type` |
| 快捷键 | `tell app "System Events" to keystroke "c" using command down` | `desktop/shortcut` |
| 点击菜单 | `tell app "System Events" to click menu item "New Tab" of menu "File"` | `desktop/menu_click` |
| 截图 | `screencapture -x /tmp/kyber_desktop.png` | `desktop/screenshot` |
| 读取 UI | `tell process "Safari" to get value of text field 1 of window 1` | `desktop/read_ui` |

## 实现步骤

### Step 1: Desktop 工具层（`tools.rs` 新增部分）

```rust
// Desktop actions routed through execute_action:
"desktop_screenshot" → screencapture -x <path>
"desktop_type"       → osascript keystroke
"desktop_click"      → osascript click at coordinates
"desktop_focus"      → osascript activate app
"desktop_shortcut"   → osascript keystroke with modifiers
"desktop_get_ui"     → osascript get UI element value
```

### Step 2: 安全层适配

桌面操控比浏览器更危险——它能点任何按钮、输入任何文本。Kyber 的安全层已经覆盖了 `execute`（终端命令），所有桌面操作走 `execute_command` → 安全层的 `needs_confirm` 和 `circuit_breaker` 自然生效。

桌面特有风险：
- 误操作桌面文件 → 安全层 `require_confirm` 已覆盖
- 无限循环按键 → 熔断机制已覆盖
- 读取隐私数据 → 观测器探测异常 `execute` 密度

### Step 3: 控制器 prompt 更新

在 controller 的系统提示中加入 desktop 操作说明，LLM 自然知道何时用。

## 工程量

| 项目 | 时间 |
|---|---|
| Desktop 工具层 | 2h |
| 安全层适配 | 0.5h（已验证无需额外代码） |
| Controller prompt | 0.5h |
| 测试（手动） | 1h |

**总计：约 4 小时**

## 优先级

低。当前 `execute` + `osascript` 已经可以间接操控桌面。原生桌面操控的价值在于让 LLM 自己决定何时用，而非用户手动写 AppleScript。
