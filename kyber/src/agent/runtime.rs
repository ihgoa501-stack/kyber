use colored::Colorize;
use super::observer::Observer;
use super::controller::Controller;
use super::tools::Tools;
use super::safety_layer::SafetyLayer;
use super::llm::Backend;

/// Main control loop — the heart of Kyber Agent.
/// Controller and observer use independent LLM backends (Separation Principle).
///
/// Controller backend: KYBER_CONTROLLER_API_KEY, KYBER_CONTROLLER_PROVIDER, KYBER_CONTROLLER_MODEL
/// Observer backend:   KYBER_OBSERVER_API_KEY, KYBER_OBSERVER_PROVIDER, KYBER_OBSERVER_MODEL
///
/// Falls back to KYBER_CONTROLLER_* if only one is configured (single-LLM mode).
pub async fn run(
    task: String,
    max_iterations: u32,
    confidence_threshold: f64,
    observer_provider: Option<String>,
    observer_model: Option<String>,
) -> anyhow::Result<()> {
    // Controller backend
    let controller_backend = Backend::from_env("controller")?;

    // Observer backend — if explicitly configured, use it. Otherwise reuse controller.
    let observer_backend = if std::env::var("KYBER_OBSERVER_API_KEY").is_ok() {
        let mut b = Backend::from_env("observer")?;
        b.name = "observer".into();
        if let Some(p) = observer_provider {
            b.provider = match p.as_str() {
                "openai" => super::llm::Provider::OpenAI,
                _ => super::llm::Provider::Anthropic,
            };
        }
        if let Some(m) = observer_model {
            b.model = m;
        }
        b
    } else {
        // Fall back to controller backend — single LLM mode, but with a note
        let mut b = controller_backend.clone();
        b.name = "observer (shared with controller)".into();
        b
    };

    let single_llm = std::env::var("KYBER_OBSERVER_API_KEY").is_err();
    if single_llm {
        println!("{} 控制器和观测器共享同一个 LLM（设置 KYBER_OBSERVER_API_KEY 启用分离）", "ℹ".dimmed());
    }

    let mut safety = SafetyLayer::new(max_iterations);
    let mut observer = Observer::new(confidence_threshold, observer_backend.clone());
    let mut controller = Controller::new(max_iterations, task.clone(), controller_backend.clone());
    let tools = Tools::new();

    println!("\n{} Kyber Agent 已启动", "═══".cyan().bold());
    println!("任务: {}", task);
    println!("最大步数: {}", max_iterations);
    println!("置信度门限: {}", confidence_threshold);
    println!("控制器: [{}] {} ({})",
        match controller_backend.provider { super::llm::Provider::Anthropic => "Anthropic", super::llm::Provider::OpenAI => "OpenAI" },
        controller_backend.model,
        controller_backend.name,
    );
    println!("观测器: [{}] {} ({})\n",
        match observer_backend.provider { super::llm::Provider::Anthropic => "Anthropic", super::llm::Provider::OpenAI => "OpenAI" },
        observer_backend.model,
        observer_backend.name,
    );

    // Set initial context
    observer.add_context(format!("任务: {}", task));

    loop {
        safety.advance();
        println!("[步 {}]", safety.iteration_count);

        // 1. Observe
        let observation = observer.observe().await;
        println!("  置信度: {:.2} — {}", observation.confidence, observation.summary);

        // 2. Confidence gate
        if observation.confidence < confidence_threshold {
            println!("  {} 置信度偏低，请示用户", "⚠".yellow());
            println!("  问题: {}", observation.summary);
            print!("  请指导: ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut hint = String::new();
            std::io::stdin().read_line(&mut hint).ok();
            let hint = hint.trim().to_string();
            if !hint.is_empty() {
                observer.add_context(format!("用户指导: {}", hint));
            }
            continue;
        }

        // 3. Decide
        let action = controller.decide(&observation).await;
        println!("  决策: {}", action);

        if controller.is_done() {
            println!("\n{} 任务完成", "✓".green());
            safety.print_report();
            break;
        }

        // 4. Safety check — pre-verify
        if action.needs_confirm() {
            print!("  执行 [{}]? [Y/n]: ", action.kind);
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() == "n" {
                println!("  已取消");
                observer.add_context(format!("用户拒绝: [{}] {}", action.kind, action.description));
                continue;
            }
        }

        // 5. Execute
        println!("  执行: {}", action.description);
        let result = tools.execute_action(&action);
        match &result {
            Ok(out) => println!("  结果: {} 字节", out.len()),
            Err(e) => println!("  失败: {}", e),
        }

        // 6. Record
        let success = result.is_ok();
        observer.add_context(format!(
            "[步 {}] {} → {}", safety.iteration_count, action, if success { "ok" } else { "fail" }
        ));

        let ok = safety.record(&action.kind, success);
        if !ok {
            println!("  {} 熔断触发！", "✗".red());
            safety.print_report();
            break;
        }

        // 7. Handle failure
        if !success {
            controller.handle_failure(&action);
        }

        // 8. Check termination
        if controller.is_done() || safety.should_terminate() {
            if safety.should_terminate() {
                println!("\n{} 达到上限 ({} 步)，停止。", "■".red(), max_iterations);
            } else {
                println!("\n{} 任务完成", "✓".green());
            }
            safety.print_report();
            break;
        }
    }

    Ok(())
}
