/// L3 Strategic Planner — breaks user tasks into executable sub-tasks.
///
/// Operates at the "plan" timescale (slow, conservative).
/// Plans once before execution; re-plans only on sub-task failure.
use colored::Colorize;
use super::llm::{Backend, call};
use super::controller::Controller;
use super::observer::Observer;
use super::tools::Tools;
use super::safety_layer::SafetyLayer;
use super::adaptation::AdaptationState;

#[derive(Debug, Clone)]
pub struct Plan {
    pub original_task: String,
    pub subtasks: Vec<SubTask>,
}

#[derive(Debug, Clone)]
pub struct SubTask {
    pub id: u32,
    pub description: String,
    pub status: SubTaskStatus,
    pub result: Option<String>,
    pub steps_used: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SubTaskStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
}

/// Generate a plan from the user's task using LLM.
pub async fn generate_plan(backend: &Backend, task: &str) -> anyhow::Result<Plan> {
    let sys = r#"你是 Kyber 的战略规划器。将用户任务分解为子任务，输出 JSON:
{
  "subtasks": [
    {"id": 1, "description": "第一步做什么", "max_steps": 5},
    {"id": 2, "description": "第二步做什么", "max_steps": 5}
  ]
}
规则:
- 每个子任务应该是可独立执行的原子操作
- 最多 3 个子任务。简单任务 1 个就够了
- 不要输出除 JSON 以外的任何内容"#;

    let response = call(backend, sys, task).await?;
    let json_str = extract_json(&response)
        .ok_or_else(|| anyhow::anyhow!("规划器未返回有效 JSON"))?;

    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    let arr = parsed["subtasks"].as_array()
        .ok_or_else(|| anyhow::anyhow!("规划器返回了无效的子任务列表"))?;

    let subtasks: Vec<SubTask> = arr.iter().map(|st| SubTask {
        id: st["id"].as_u64().unwrap_or(0) as u32,
        description: st["description"].as_str().unwrap_or("未知").into(),
        status: SubTaskStatus::Pending,
        result: None,
        steps_used: 0,
    }).collect();

    if subtasks.is_empty() {
        // No plan needed — single task, execute directly
        return Ok(Plan {
            original_task: task.into(),
            subtasks: vec![SubTask {
                id: 1,
                description: task.into(),
                status: SubTaskStatus::InProgress,
                result: None,
                steps_used: 0,
            }],
        });
    }

    Ok(Plan { original_task: task.into(), subtasks })
}

/// Execute a single sub-task using the tactical control loop (L2).
pub async fn execute_subtask(
    subtask: &mut SubTask,
    controller_backend: &Backend,
    observer_backend: &Backend,
    confidence_threshold: f64,
    tools: &Tools,
    budget: u32,
) {
    subtask.status = SubTaskStatus::InProgress;
    let max_iterations = budget;

    let mut safety = SafetyLayer::new(max_iterations);
    let mut observer = Observer::new(confidence_threshold, observer_backend.clone(), &subtask.description);
    let mut controller = Controller::new(max_iterations, subtask.description.clone(), controller_backend.clone());
    let mut adaptation = AdaptationState::new();

    observer.add_context(format!("子任务: {}", subtask.description));

    println!("\n{} L2 执行: {}", "──".cyan(), subtask.description.cyan());
    println!("  预算: {} 步", max_iterations);

    loop {
        safety.advance();

        let observation = observer.observe().await;
        let mode = adaptation.update(observation.confidence);

        // Show adaptation status with signal breakdown
        let mode_str: String = match mode {
            super::adaptation::OperatingMode::Aggressive => "⚡".into(),
            super::adaptation::OperatingMode::Nominal => "●".into(),
            super::adaptation::OperatingMode::Conservative => "⚠".into(),
            super::adaptation::OperatingMode::Safe => "■".into(),
        };
        print!("  {}", mode_str);
        for (name, score, _) in &observation.breakdown {
            let c = if *score > 0.7 { format!("{}", name).green() }
                else if *score < 0.4 { format!("{}", name).red() }
                else { format!("{}", name).yellow() };
            print!(" {:.0}%{}", score * 100.0, c);
        }
        println!();

        // Double-observe in conservative/safe mode (now actually wired)
        if adaptation.should_double_observe() {
            let obs2 = observer.observe().await;
            println!("  二次观测: {:.2}", obs2.confidence);
        }

        // How 1: adaptation-driven early termination
        if matches!(adaptation.mode, super::adaptation::OperatingMode::Safe) {
            subtask.status = SubTaskStatus::Failed("适应器切换至安全模式".into());
            subtask.steps_used = safety.iteration_count;
            println!("  {} 安全模式: 终止子任务", "■".red());
            return;
        }
        if matches!(adaptation.mode, super::adaptation::OperatingMode::Conservative)
            && safety.iteration_count > max_iterations / 2
        {
            subtask.status = SubTaskStatus::Failed("保守模式下提前终止".into());
            subtask.steps_used = safety.iteration_count;
            println!("  {} 保守模式: 提前终止", "⚠".yellow());
            return;
        }

        if observation.confidence < confidence_threshold {
            if adaptation.mode == super::adaptation::OperatingMode::Safe {
                // In safe mode and low confidence → fail this sub-task gracefully
                subtask.status = SubTaskStatus::Failed(
                    format!("置信度过低 ({:.2})", observation.confidence));
                subtask.steps_used = safety.iteration_count;
                println!("  {} 子任务失败: 置信度过低", "✗".red());
                return;
            }
            // Otherwise retry with adaptation
            observer.add_context(format!("低置信度: {}", observation.summary));
            continue;
        }

        let action = controller.decide(&observation).await;
        println!("  L2[{}] {}", safety.iteration_count, action);

        if controller.is_done() {
            subtask.status = SubTaskStatus::Complete;
            subtask.result = Some(observation.summary.clone());
            subtask.steps_used = safety.iteration_count;
            println!("  {} 子任务完成", "✓".green());
            return;
        }

        // Adaptation-modulated safety: skip dangerous actions in unsafe modes
        if action.needs_confirm() {
            if matches!(mode, super::adaptation::OperatingMode::Safe) {
                observer.add_context(format!("安全模式: 跳过高风险 [{}]", action.kind));
                println!("  {} 安全模式跳过 [{}]", "■".red(), action.kind);
                continue;
            }
            if matches!(mode, super::adaptation::OperatingMode::Conservative) {
                println!("  {} 保守模式执行 [{}]", "⚠".yellow(), action.kind);
            }
        }

        let result = tools.execute_action(&action);
        let success = result.is_ok();
        let output_len = result.as_ref().map(|s| s.len()).unwrap_or(0);

        // How 2: quality scoring — short useless output = bad signal
        let is_low_quality = success && output_len < 20
            && matches!(action.kind.as_str(), "execute" | "respond");
        let effective_success = success && !is_low_quality;

        observer.record_step(effective_success, &action.kind, output_len);

        // Feed tool output directly into controller's next decision
        let output_preview = match &result {
            Ok(out) => out.clone(),
            Err(e) => e.clone(),
        };
        controller.last_result = Some(output_preview.clone());

        // Trim for observer context
        let context_preview = if output_preview.len() > 500 {
            format!("{}... ({})", &output_preview[..500], output_preview.len())
        } else {
            output_preview.clone()
        };
        observer.add_context(format!(
            "L2[{}] {} → {} | 输出: {}",
            safety.iteration_count, action,
            if success { "ok" } else { "fail" },
            context_preview,
        ));

        let ok = safety.record(&action.kind, success);
        if !ok {
            subtask.status = SubTaskStatus::Failed("熔断触发".into());
            subtask.steps_used = safety.iteration_count;
            println!("  {} 熔断，子任务终止", "✗".red());
            return;
        }

        if !success {
            controller.handle_failure(&action);
        }

        if safety.should_terminate() || controller.is_done() {
            if safety.should_terminate() {
                subtask.status = SubTaskStatus::Failed("超过步数限制".into());
                subtask.steps_used = safety.iteration_count;
                println!("  {} 子任务超限", "✗".red());
            } else {
                subtask.status = SubTaskStatus::Complete;
                subtask.result = Some(observation.summary.clone());
                subtask.steps_used = safety.iteration_count;
                println!("  {} 子任务完成", "✓".green());
            }
            return;
        }
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
