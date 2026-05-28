use colored::Colorize;
use super::observer::Observer;
use super::controller::Controller;
use super::tools::Tools;
use super::safety_layer::SafetyLayer;

/// Main control loop — the heart of Kyber Agent.
/// observe → confidence gate → decide → safety → execute → verify → record → loop
pub async fn run(task: String, max_iterations: u32, confidence_threshold: f64) -> anyhow::Result<()> {
    // Check for API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("{} 需要设置 ANTHROPIC_API_KEY 环境变量才能运行", "⚠".yellow());
        println!("  export ANTHROPIC_API_KEY=sk-ant-...");
        return Ok(());
    }

    let mut safety = SafetyLayer::new(max_iterations);
    let mut observer = Observer::new(confidence_threshold);
    let mut controller = Controller::new(max_iterations, task.clone());
    let tools = Tools::new();

    println!("\n{} Kyber Agent 已启动", "═══".cyan().bold());
    println!("任务: {}", task);
    println!("最大步数: {}", max_iterations);
    println!("置信度门限: {}\n", confidence_threshold);

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
