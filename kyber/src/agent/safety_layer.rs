use std::time::{Duration, Instant};

pub struct SafetyLayer {
    pub iteration_count: u32,
    pub max_iterations: u32,
    failure_count: u32,
    failure_window: Vec<Instant>,
    circuit_breaker_threshold: u32,
    circuit_breaker_window_secs: u64,
    circuit_open: bool,
    pub audit_log: Vec<AuditEntry>,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub step: u32,
    pub action: String,
    pub result: String,
    pub timestamp: std::time::SystemTime,
}

impl SafetyLayer {
    pub fn new(max_iterations: u32) -> Self {
        SafetyLayer {
            iteration_count: 0,
            max_iterations,
            failure_count: 0,
            failure_window: Vec::new(),
            circuit_breaker_threshold: 3,
            circuit_breaker_window_secs: 60,
            circuit_open: false,
            audit_log: Vec::new(),
        }
    }

    pub fn should_terminate(&self) -> bool {
        self.iteration_count >= self.max_iterations || self.circuit_open
    }

    pub fn advance(&mut self) {
        self.iteration_count += 1;
    }

    pub fn record(&mut self, action: &str, success: bool) -> bool {
        self.audit_log.push(AuditEntry {
            step: self.iteration_count,
            action: action.to_string(),
            result: if success { "ok".into() } else { "fail".into() },
            timestamp: std::time::SystemTime::now(),
        });

        if !success {
            self.failure_count += 1;
            self.failure_window.push(Instant::now());
            let cutoff = Instant::now() - Duration::from_secs(self.circuit_breaker_window_secs);
            self.failure_window.retain(|t| *t > cutoff);
            if self.failure_window.len() as u32 >= self.circuit_breaker_threshold {
                self.circuit_open = true;
                return false;
            }
        }
        true
    }

    pub fn print_report(&self) {
        println!("\n═══ Kyber 审计报告 ═══");
        println!("执行步数: {}", self.iteration_count);
        println!("失败次数: {}", self.failure_count);
        println!("熔断状态: {}", if self.circuit_open { "已触发" } else { "正常" });
        println!("\n操作日志:");
        for entry in &self.audit_log {
            println!("  [步 {}] {} → {}", entry.step, entry.action, entry.result);
        }
    }
}
