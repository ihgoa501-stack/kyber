use dialoguer::{Confirm, Input, MultiSelect, Select};
use colored::Colorize;
use crate::config::types::*;

pub fn run_dialogue(name: &str, template: String) -> DialogueAnswers {
    println!("\n{}", "═══ 设计你的 Agent ═══".cyan().bold());
    println!("我会问你几个问题，帮你搭建一个可控可靠的 Agent。\n");

    // Round 1: Role
    let role = round_role();
    println!();

    // Round 2: Tools
    let tools = round_tools();
    println!();

    // Round 3: Risk level
    let risk_level = round_risk();
    println!();

    // Round 4: Confirmation preferences
    let confirm_actions = round_confirm(&risk_level);
    println!();

    // Round 5: Architecture
    let architecture = if Confirm::new()
        .with_prompt(format!("使用 {} 架构（默认）？还是改用深度验证架构？回答 No 切换", template))
        .default(true)
        .interact()
        .unwrap()
    {
        match template.as_str() {
            "deep-verify" => ArchitectureStyle::DeepVerify,
            _ => ArchitectureStyle::React,
        }
    } else {
        match template.as_str() {
            "react" => ArchitectureStyle::DeepVerify,
            _ => ArchitectureStyle::React,
        }
    };
    println!();

    let answers = DialogueAnswers {
        name: name.to_string(),
        role,
        tools,
        risk_level,
        confirm_actions,
        architecture,
    };

    // Round 6: Summary & confirmation
    print_summary(&answers);
    let confirmed = Confirm::new()
        .with_prompt("确认生成？")
        .default(true)
        .interact()
        .unwrap();

    if !confirmed {
        println!("已取消。");
        std::process::exit(0);
    }

    answers
}

fn round_role() -> String {
    let roles = vec![
        "软件工程师（写代码、改代码、查文档）",
        "桌面助手（操作系统、管理文件）",
        "浏览器助手（上网查信息、填表单）",
        "自定义",
    ];

    let selection = Select::new()
        .with_prompt("这个 Agent 的主要角色是什么？")
        .items(&roles)
        .default(0)
        .interact()
        .unwrap();

    match selection {
        0 => "软件工程师".into(),
        1 => "桌面助手".into(),
        2 => "浏览器助手".into(),
        3 => Input::<String>::new()
            .with_prompt("请输入角色")
            .interact_text()
            .unwrap(),
        _ => unreachable!(),
    }
}

fn round_tools() -> Vec<Tool> {
    let tool_items = vec![
        "终端命令",
        "文件系统",
        "浏览器",
        "桌面应用",
        "网络请求",
        "git 操作",
    ];

    let selections = MultiSelect::new()
        .with_prompt("允许 Agent 使用哪些工具？（空格选择，回车确认）")
        .items(&tool_items)
        .interact()
        .unwrap();

    let tools: Vec<Tool> = selections.iter().map(|i| match i {
        0 => Tool::Terminal,
        1 => Tool::Filesystem,
        2 => Tool::Browser,
        3 => Tool::Desktop,
        4 => Tool::Network,
        5 => Tool::Git,
        _ => unreachable!(),
    }).collect();

    if tools.is_empty() {
        println!("{} 至少需要选择一个工具。", "⚠".yellow());
        round_tools()
    } else {
        tools
    }
}

fn round_risk() -> RiskLevel {
    let risks = vec!["低 —— 试试无所谓，错了无妨", "中 —— 有点麻烦", "高 —— 出错很严重"];

    let selection = Select::new()
        .with_prompt("Agent 犯错的后果严重吗？")
        .items(&risks)
        .default(1)
        .interact()
        .unwrap();

    match selection {
        0 => RiskLevel::Low,
        1 => RiskLevel::Medium,
        2 => RiskLevel::High,
        _ => unreachable!(),
    }
}

fn round_confirm(risk: &RiskLevel) -> Vec<ActionKind> {
    let actions = vec![
        "读文件",
        "写文件",
        "删除文件",
        "执行命令",
        "git 操作",
        "网络请求",
    ];

    let defaults: Vec<bool> = match risk {
        RiskLevel::Low => vec![false, false, true, false, false, false],
        RiskLevel::Medium => vec![false, true, true, true, false, false],
        RiskLevel::High => vec![true, true, true, true, true, true],
    };

    let selections = MultiSelect::new()
        .with_prompt("哪些操作需要你点头确认才能执行？")
        .items(&actions)
        .defaults(&defaults)
        .interact()
        .unwrap();

    selections.iter().map(|i| match i {
        0 => ActionKind::Read,
        1 => ActionKind::Write,
        2 => ActionKind::Delete,
        3 => ActionKind::Execute,
        4 => ActionKind::Git,
        5 => ActionKind::Network,
        _ => unreachable!(),
    }).collect()
}

fn print_summary(answers: &DialogueAnswers) {
    println!("\n{}", "═══ 你的 Agent 配置 ═══".cyan().bold());
    println!("名称：     {}", answers.name);
    println!("角色：     {}", answers.role);
    println!("风险等级： {}", match answers.risk_level {
        RiskLevel::Low => "低",
        RiskLevel::Medium => "中",
        RiskLevel::High => "高",
    });
    println!("工具：     {}", answers.tools.iter()
        .map(|t| format!("{:?}", t))
        .collect::<Vec<_>>()
        .join(", "));

    let (threshold, max_iter, breaker) = compute_safety_params(&answers.risk_level, &answers.architecture);
    println!("\n{}", "你会得到这些保障：".green());
    println!("  ✓ 置信度低于 {} 时停住问人", threshold);
    println!("  ✓ 连续 {} 自动熔断", breaker);
    println!("  ✓ 最多 {} 步迭代，不会死循环", max_iter);
    println!("  ✓ 每步都有审计日志\n");
}

pub fn compute_safety_params(risk: &RiskLevel, arch: &ArchitectureStyle) -> (f64, u32, &'static str) {
    let base = match risk {
        RiskLevel::Low => (0.4, 50, "60 秒内 5 次失败"),
        RiskLevel::Medium => (0.6, 25, "60 秒内 3 次失败"),
        RiskLevel::High => (0.8, 15, "60 秒内 2 次失败"),
    };
    if matches!(arch, ArchitectureStyle::DeepVerify) {
        (base.0, base.1 * 2, base.2)
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_params_low_risk() {
        let (thresh, max_iter, _) = compute_safety_params(&RiskLevel::Low, &ArchitectureStyle::React);
        assert_eq!(thresh, 0.4);
        assert_eq!(max_iter, 50);
    }

    #[test]
    fn test_safety_params_high_risk() {
        let (thresh, max_iter, _) = compute_safety_params(&RiskLevel::High, &ArchitectureStyle::React);
        assert_eq!(thresh, 0.8);
        assert_eq!(max_iter, 15);
    }

    #[test]
    fn test_safety_params_deep_verify_gives_more_iterations() {
        let (_, max_iter_react, _) = compute_safety_params(&RiskLevel::Medium, &ArchitectureStyle::React);
        let (_, max_iter_dv, _) = compute_safety_params(&RiskLevel::Medium, &ArchitectureStyle::DeepVerify);
        assert!(max_iter_dv > max_iter_react);
    }
}
