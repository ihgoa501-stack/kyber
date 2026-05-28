/// Main control loop — hierarchical (L3 → L2 → L1).
///
/// L3 (Strategic): Planner generates sub-tasks from user task (slow, conservative).
/// L2 (Tactical):  Each sub-task runs its own control loop (observe→decide→execute).
/// L1 (Operational): Tools execute atomic actions.
use colored::Colorize;
use super::tools::Tools;
use super::planner;
use super::llm::Backend;

pub async fn run(
    task: String,
    max_iterations: u32,
    confidence_threshold: f64,
    observer_provider: Option<String>,
    observer_model: Option<String>,
) -> anyhow::Result<()> {
    let controller_backend = Backend::from_env("controller")?;

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
        let mut b = controller_backend.clone();
        b.name = "observer (shared with controller)".into();
        b
    };

    if std::env::var("KYBER_OBSERVER_API_KEY").is_err() {
        println!("{} 控制器和观测器共享同一个 LLM", "ℹ".dimmed());
    }

    println!("\n{} Kyber Agent 已启动 (分层控制)", "═══".cyan().bold());
    println!("任务: {}", task);
    println!("控制器: [{}] {}", controller_backend.model, controller_backend.name);
    println!("观测器: [{}] {}\n", observer_backend.model, observer_backend.name);

    // ── L3: Strategic Planning ──
    println!("{} 战略规划 (L3)", "┌─".cyan());
    let mut plan = planner::generate_plan(&controller_backend, &task).await?;

    println!("│ 计划: {} 个子任务", plan.subtasks.len());
    for st in &plan.subtasks {
        println!("│   {}. {}", st.id, st.description);
    }
    println!("{}", "└─".cyan());

    let tools = Tools::new();
    let mut total_steps = 0u32;

    // ── L2: Execute each sub-task ──
    let mut completed = 0u32;
    let mut failed = 0u32;
    let subtask_count = plan.subtasks.len();

    for i in 0..subtask_count {
        if total_steps >= max_iterations {
            println!("\n{} 达到最大步数 ({}), 停止。", "■".red(), max_iterations);
            break;
        }

        // Execute one sub-task
        planner::execute_subtask(
            &mut plan.subtasks[i],
            &controller_backend,
            &observer_backend,
            confidence_threshold,
            &tools,
        ).await;

        total_steps += plan.subtasks[i].steps_used;

        match &plan.subtasks[i].status {
            planner::SubTaskStatus::Complete => {
                completed += 1;
            }
            planner::SubTaskStatus::Failed(reason) => {
                failed += 1;
                println!("  {} 子任务 {} 失败: {}", "■".red(), plan.subtasks[i].id, reason);

                // L3 re-planning: ask LLM if remaining tasks need adjustment
                if i + 1 < subtask_count {
                    let context = format!(
                        "原任务: {}\n已完成: {}\n失败: 子任务 {} ({})\n剩余: {:?}\n需要调整计划吗? 输出 JSON: {{\"adjust\": true, \"new_subtasks\": [{{\"id\": {}, \"description\": \"...\", \"max_steps\": 5}}]}} 或 {{\"adjust\": false}}",
                        task,
                        plan.subtasks.iter().filter(|s| matches!(s.status, planner::SubTaskStatus::Complete)).map(|s| s.description.as_str()).collect::<Vec<_>>().join("; "),
                        plan.subtasks[i].id,
                        reason,
                        &plan.subtasks[i+1..].iter().map(|s| s.description.as_str()).collect::<Vec<_>>(),
                        plan.subtasks.last().map(|s| s.id).unwrap_or(0) + 1,
                    );

                    let replan_sys = "你是战略规划器。评估是否需要在失败后调整计划。只输出 JSON。";
                    if let Ok(response) = super::llm::call(&controller_backend, replan_sys, &context).await {
                        if let Some(json) = extract_json(&response) {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                                if parsed.get("adjust").and_then(|v| v.as_bool()).unwrap_or(false) {
                                    if let Some(new_sts) = parsed.get("new_subtasks").and_then(|v| v.as_array()) {
                                        let adjusted: Vec<planner::SubTask> = new_sts.iter().enumerate().map(|(j, st)| {
                                            planner::SubTask {
                                                id: plan.subtasks[i].id + j as u32 + 1,
                                                description: st["description"].as_str().unwrap_or("").into(),
                                                status: planner::SubTaskStatus::Pending,
                                                result: None,
                                                steps_used: 0,
                                            }
                                        }).collect();
                                        println!("  {} 计划已调整: {} 个新子任务", "↻".yellow(), adjusted.len());
                                        plan.subtasks.truncate(i + 1);
                                        plan.subtasks.extend(adjusted);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Final Report ──
    println!("\n{} 分层执行报告 ═══", "═══".cyan().bold());
    println!("总步数: {}", total_steps);
    println!("完成: {} / 失败: {}", completed, failed);
    for st in &plan.subtasks {
        let status = match &st.status {
            planner::SubTaskStatus::Complete => "✓".green(),
            planner::SubTaskStatus::Failed(_) => "✗".red(),
            _ => "…".dimmed(),
        };
        println!("  {} [{}] {} ({} 步)", status, st.id, st.description, st.steps_used);
    }

    Ok(())
}

fn extract_json(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Some(text[start..=end].to_string());
        }
    }
    None
}
