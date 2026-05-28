---
status: Draft
inspired-by: Qian Xuesen Engineering Cybernetics (1954)
---

# Kyber CLI — A Control-Engineered Design

> This document is the design specification for the Kyber CLI tool.
> It is itself written through the lens of Engineering Cybernetics —
> the 8 lenses applied to the system that builds systems.

---

## Table of Contents

1. [System Identification](#1-system-identification)
2. [Feedback Loop Design](#2-feedback-loop-design)
3. [Stability Analysis](#3-stability-analysis)
4. [Controllability](#4-controllability)
5. [Observability](#5-observability)
6. [Uncertainty Handling](#6-uncertainty-handling)
7. [Fault Tolerance](#7-fault-tolerance)
8. [Hierarchical Control](#8-hierarchical-control)

---

## 1. System Identification

### 1.1 What Is Kyber CLI as a System?

Kyber CLI is not a code generator. It is a **controller manufacturing system**.

```
                Kyber CLI is here:
                                        ┌─────────────────────┐
                                        │    Kyber CLI        │
  User ──┬─ goal ──────────────────────►│  (meta-controller)  │
          │                             │                     │
          │                             │  generates a custom │
          │  ◄─ output / error ─────────│  controller (Agent) │
          │                             └──────────┬──────────┘
          │                                        │
          │                                        ▼
          │                             ┌─────────────────────┐
          │                             │   Generated Agent   │
          │                             │   (controller)      │
          │  status                     │                     │
          │  ◄──────────────────────────│  controls the real  │
          │                             │  world on user's    │
          │  input                      │  behalf             │
          │ ───────────────────────────►│                     │
          │                             └─────────────────────┘
                                                    │
                                                    ▼
                                          ┌─────────────────────┐
                                          │    Real World       │
                                          │  (controlled plant) │
                                          └─────────────────────┘
```

This is a **two-layer control hierarchy** — Kyber is the meta-controller that
manufactures the controller that operates on the world.

### 1.2 Control Objective

**The objective of Kyber CLI**: produce an Agent whose behavior is stable,
predictable, and verifiable — not just at generation time, but at runtime.

```
Primary objective:  output_agent.behavior @ (stable ∩ predictable ∩ verifiable)
Secondary:          minimize user's uncertainty about the output_agent
Constraint:         generation completes in bounded time
```

This is fundamentally different from a code scaffold. A scaffold's objective
is "produce syntactically valid files." Kyber's objective is **behavioral**.

### 1.3 State Space

The state of the CLI during a session:

```
ẋ(t) = A·x(t) + B·u(t) + w(t)

State variables:
  x₁: user_intent_clarity     ∈ [0, 1]  — how well we understand what user wants
  x₂: risk_profile_resolution ∈ [0, 1]  — how precisely risk is mapped
  x₃: tool_set_completeness   ∈ [0, 1]  — whether all required tools are enumerated
  x₄: dialogue_completeness   ∈ [0, 1]  — how many rounds have converged
  x₅: pending_changes         ∈ {0, 1}  — whether user made modifications in preview
```

**Initial state** (user types `kyber init`):
```
x(0) = [0.0, 0.0, 0.0, 0.0, 0.0]
```
All variables at zero — we know nothing.

**Target state** (ready to generate):
```
x(target) = [>0.9, >0.9, >0.9, 1.0, 0.0]
```
All dimensions sufficiently resolved.

### 1.4 Control Inputs

The CLI's control inputs — what it can do to the environment:

```
u ∈ {
  ask_question(round_n)    —  present a specific question to user
  show_summary()           —  display current configuration
  request_confirmation()   —  ask Y/N to proceed
  generate_project()       —  write files to disk
  abort()                  —  exit without generating
}
```

### 1.5 Measured Outputs

What the CLI observes:

```
y₁: user_response_content  —  the actual text/selection user provided
y₂: confirmation_signal    —  Y/N, yes it's ready to proceed
y₃: modification_request   —  user wants to change previous answers
y₄: abort_signal           —  user Ctrl+C / exits early
```

### 1.6 Disturbances

What the CLI cannot control:

```
w₁: ambiguity_in_user_input  —  "I want an AI that's helpful" is too vague
w₂: user_uncertainty         —  user doesn't know what tools they need
w₃: user_changes_mind        —  user says one thing, then wants different
w₄: environmental_failure    —  file system full, permissions denied
```

### 1.7 The Essential Feedback Loop

Kyber CLI's core loop — every iteration must reduce uncertainty:

```
              x(t) = current state
                  │
                  ▼
          select question
          for round x₄+1
                  │
                  ▼
          present to user
                  │
                  ▼
          observe y(t)       ──── disturbance w(t) ────►
                  │
                  ▼
          update state:
          x(t+1) = f(x(t), y(t))
                  │
                  ▼
          check convergence:
          x(t+1) > threshold? ──no──► repeat
                  │ yes
                  ▼
          generate Agent
```

The convergence condition: **for each dimension in state space, show monotonic
decrease in uncertainty.**

```
on each round t:
  uncertainty(x₁) must decrease:  entropy(x₁(t+1)) ≤ entropy(x₁(t))
  uncertainty(x₂) must decrease:  entropy(x₂(t+1)) ≤ entropy(x₂(t))
  ...

If entropy increases on any dimension → system is unstable → add another round
or provide clarification.
```

---

## 2. Feedback Loop Design

### 2.1 Primary Loop: The Dialogue

The 6-round dialogue is the CLI's **primary feedback loop**:

```
Round 1: name
  y₁ = user_provides_name
  effect: x₄ advances, identity established

Round 2: role
  y₂ = user_picks_role
  effect: x₁ jumps to > 0.7 (role narrows the goal space significantly)

Round 3: tools
  y₃ = user_selects_tools
  effect: x₃ → 1.0 (tools are known)

Round 4: risk
  y₄ = user_picks_risk_level
  effect: x₂ → 1.0 (risk is resolved)

Round 5: confirmation preferences
  y₅ = user_selects_confirm_actions
  effect: all dimensions should now be > 0.9

Round 6: summary & Y/N confirmation
  y₆ = Y or modification_request
  if Y → generate
  if modification → loop back with updated state
```

Each round is a **measurement that reduces state uncertainty**. The loop gain
(the rate at which uncertainty decreases per round) is designed to be
sufficiently high that the system converges within 6 rounds.

### 2.2 Secondary Loop: The Verification Chain

Inside the generated Agent, Kyber embeds a **nested feedback loop**:

```
Generated Agent's loop (from main.rs):

    ┌── observe ──→ estimate confidence ──→ below threshold? ──→ ask user ──┐
    │                                                                       │
    │    above threshold                                                    │
    │         │                                                             │
    │         ▼                                                             │
    │    decide next action                                                 │
    │         │                                                             │
    │         ▼                                                             │
    │    pre-verify (simulate) ── fails? ──→ reconsider                    │
    │         │                          │                                  │
    │       passes                       ▼                                  │
    │         │                     choose different action                 │
    │         ▼                          │                                  │
    │    require_confirm? ── yes ──→ ask user                              │
    │         │                          │                                  │
    │         no                          ▼                                │
    │         │                     wait for user input                     │
    │         ▼                          │                                  │
    │    execute                        ────┐                              │
    │         │                              │                              │
    │         ▼                              │                              │
    │    post-verify result ── fails? ───────┘                              │
    │         │                                                            │
    │       ok                                                             │
    │         │                                                            │
    └─────────┘ ←──── iterate until done or safety.terminate ──────────────┘
```

This is a **multi-loop system**:
- Outer loop: the task-level iteration (observe → decide → execute)
- Inner loop 1: confidence gate (if uncertain, loop to ask)
- Inner loop 2: pre-verification (if simulation fails, re-decide)
- Inner loop 3: user confirmation gate (if dangerous, wait)
- Inner loop 4: post-verification (if result wrong, retry or re-decide)

### 2.3 Gain Scheduling

The "gain" of the generated Agent's controller varies by risk level:

```
RiskLevel::Low:
  gain = 0.9   (acts fast, confirms rarely)
  observer confidence_threshold = 0.4
  safety require_confirm = [Delete]
  safety circuit_breaker = 5/60

RiskLevel::Medium:
  gain = 0.6
  observer confidence_threshold = 0.6
  safety require_confirm = [Delete, Write, Execute]
  safety circuit_breaker = 3/60

RiskLevel::High:
  gain = 0.3   (acts slow but safe)
  observer confidence_threshold = 0.8
  safety require_confirm = All
  safety circuit_breaker = 2/60
```

**Lower gain = higher stability margin.** This is a control-theoretic design
choice, not a heuristic. The user's risk preference directly sets the loop gain.

---

## 3. Stability Analysis

### 3.1 CLI Stability: Does the Dialogue Converge?

Define an energy function for the CLI's state:

```
V(x) = 1 - (w₁·x₁² + w₂·x₂² + w₃·x₃² + w₄·x₄²) / (w₁ + w₂ + w₃ + w₄)

where w₁..w₄ are weights reflecting the relative importance of each dimension.
```

**Stability condition**: V(x) must strictly decrease with each round, and
converge to V(x) < 0.1 within 6 rounds.

```
Round 0: V = 1.0     (total uncertainty)
Round 1: V ≈ 0.75    (name helps but doesn't narrow much)
Round 2: V ≈ 0.50    (role narrows goal space)
Round 3: V ≈ 0.35    (tools constrain action space)
Round 4: V ≈ 0.20    (risk sets gain and safety parameters)
Round 5: V ≈ 0.10    (confirm preferences finalize)
Round 6: V ≈ 0.05    (Y/N confirmation, final check)
→ System converges within spec
```

**Instability scenarios**:

| Scenario | Detection | Response |
|---|---|---|
| User keeps changing answers | V does not decrease | CLI offers to restart or suggests defaults |
| User provides contradictory answers | V increases on some dimension | CLI flags the contradiction explicitly |
| User doesn't know what to choose | V stalls on a dimension | CLI provides more explanation or example |

### 3.2 Temporal Coupling Analysis

The dialogue rounds must be **temporally decoupled** — each round must reduce
uncertainty on its target dimension without increasing uncertainty on others.

```
Coupling matrix (how round j affects dimension i):

              Round:  name  role  tools  risk  confirm
                    ┌─────────────────────────────┐
  dim: intent      │ 0.1   0.7   0.2   0.0   0.0 │  (rows sum to ≤ 1)
  dim: risk        │ 0.0   0.1   0.1   0.8   0.0 │
  dim: tools       │ 0.0   0.0   0.9   0.0   0.1 │
  dim: completeness│ 0.2   0.2   0.2   0.2   0.2 │
                    └─────────────────────────────┘
```

**Design rule**: Each row should have exactly one dominant cell (> 0.7).
If a dimension is affected by multiple rounds → those rounds are coupled. If
two rounds both affect the same dimension → they must have different timescales.

### 3.3 Nyquist Analogue for the Dialogue Loop

The dialogue feedback loop is stable if:

```
For each user response y(t):
  The resulting state update Δx(t) must be in the same direction as the
  intended correction, with magnitude less than the remaining error.

i.e., no overshoot: if intent is 30% resolved after round 2,
      round 3 should not make it "150% resolved" (overshoot and oscillate).
```

**Real-world implication**: Don't ask redundant questions that risk confusing
the user or contradicting previous answers.

---

## 4. Controllability

### 4.1 CLI Controllability: Can the User Drive the System?

A control system is **controllable** if the user can drive it from any state
to any target state in finite steps.

**For Kyber CLI**: the user's answers are the control inputs that drive the
state from `x(0) = [0,0,0,0,0]` to `x(target) = [0.9,0.9,0.9,1.0,0.0]`.

Controllability condition: **each state dimension must have at least one
question that can perturb it.**

```
x₁ (intent): affected by Round 2 (role) ✓
x₂ (risk):   affected by Round 4 (risk) ✓
x₃ (tools):  affected by Round 3 (tools) ✓
x₄ (completeness): each round advances this ✓
x₅ (pending): affected by modification in Round 6 ✓
```

All dimensions are controllable by the designed dialogue. ✓

### 4.2 Generated Agent Controllability: Can the User Steer It?

The generated Agent is controllable if the user can steer it away from
undesired behavior through available control inputs.

**Control inputs available to the user at runtime:**

```
The user can:
  - Answer "no" to confirm prompts    (low-level control)
  - Provide guidance when confidence   (mid-level control)
    is below threshold
  - Modify kyber.toml and rebuild      (high-level control)
    (adjusting confidence_threshold,
     max_iterations, require_confirm, etc.)
```

**Controllability chain**:

```
User changes kyber.toml
       │
       ▼
Agent reads new config at next restart
       │
       ▼
safety.rs adjusts behavior
       │
       ▼
observer uses new confidence_threshold
       │
       ▼
controller operates under new max_iterations
       │
       ▼
Agent's observable behavior changes
```

This chain must be **complete**. If any link is broken (e.g., changing
`confidence_threshold` in kyber.toml but safety.rs ignores it), the system
loses controllability on that dimension.

**Design rule**: Every field in kyber.toml must map to exactly one behavior
in the generated code, and the mapping must be two-way documented.

---

## 5. Observability

### 5.1 CLI Observability: Can the User See Inside?

The CLI must make its internal state visible to the user. Otherwise, the user
cannot trust the generation process.

**Observability design**:

```
CLI state → Observable to user as:

x₁ (intent)   → "Role: Software Engineer"  (shown in summary)
x₂ (risk)     → "Risk level: High"         (shown in summary)
x₃ (tools)    → "Tools: terminal, file..." (shown in summary)
x₄ (dialogue) → "Round 3/6"               (shown as progress)
x₅ (pending)  → "Changes pending"         (shown if modification pending)
```

The confirmation screen at round 6 makes the entire state fully observable:

```
═══ 你的 Agent 配置 ═══
名称：      my-coder
角色：      软件工程师
工具：      终端 + 文件系统 + git
风险：      高
确认操作：  写文件、删文件、执行命令
控制架构：  react

你会得到这些保障：
  ✓ 置信度低于 0.8 时停住问人
  ✓ 连续 60 秒内 2 次失败自动熔断
  ✓ 最多 15 步迭代，不会死循环
  ✓ 每步都有审计日志
```

### 5.2 Generated Agent Observability: Can the User See Inside the Agent?

The generated Agent's audit log is its observability mechanism. It must
contain enough information to reconstruct the Agent's full internal state.

```rust
// Each audit entry captures the full state at that step:
struct AuditEntry {
    step: u32,
    timestamp: SystemTime,

    // Internal state
    state: AgentStateSnapshot {
        confidence: f64,
        iteration_count: u32,
        context_saturation: f64,
    },

    // What the Agent observed
    observation: String,
    tool_results: Vec<ToolResult>,

    // What the Agent decided
    decision: Action,
    rationale: String,          // ← critical for trust

    // What happened
    execution_result: Result<String>,
    verification_status: VerificationOutcome,
}
```

**Observability condition**: A user reading the audit log should be able to
answer "why did the Agent do what it did?" for every step.

---

## 6. Uncertainty Handling

Kyber CLI is designed to operate in the presence of fundamental uncertainty:

### 6.1 User Ambiguity

Users cannot always precisely express what they want. The CLI handles this
with:

```
Strategy 1: Forced choice via multiple choice (Rounds 2-5)
  Instead of "What do you want?" → "Pick from these options"
  This bounds the uncertainty space.

Strategy 2: Preview and confirm (Round 6)
  Make the final state fully visible before executing.
  This catches misinterpretation.

Strategy 3: Custom input option at each round
  If no option fits, user can type freely.
  The interpreter then does fuzzy matching to map it back to state space.
```

### 6.2 Generated Agent Uncertainty

The generated Agent must operate in an uncertain environment:

```
At runtime, the Agent faces:
  - LLM hallucinations:      output that looks correct but isn't
  - Tool failures:           command returns error unexpectedly
  - Ambiguous user input:    "fix this bug" — which bug?
  - Shifting context:        user's goal changes mid-conversation

How the generated Agent handles this:
  - confidence_threshold gate:  don't act on low-certainty
  - pre-verification:           simulate before executing
  - post-verification:          check after execution
  - circuit breaker:            stop if too many errors
  - ask user:                   fall back to human when stuck
```

### 6.3 Brittleness of the Generated Code

The generated Rust code must be **robust to its own generation errors**:

```
Generated code is always:
  - Wrapped in Result<_, Error> (never panics)
  - Parsed back to verify syntax validity
  - Accompanied by a test file that compiles and runs

If template rendering fails → catch early, report precisely:
  "Failed to generate src/main.rs: variable 'tools' not in context"
```

---

## 7. Fault Tolerance

### 7.1 CLI Fault Tolerance

| Fault | Detection | Response |
|---|---|---|
| User Ctrl+C mid-dialogue | Signal handler | Save partial state to `.kyber-session.json`, tell user "you can resume" |
| File system full during generation | IO error | Clean up partial files, report disk usage, suggest alternative path |
| Template file missing | Startup check | CLI should validate all templates at startup, not at generation time |
| Dialogue state inconsistent | State validation | Reset to last consistent state, notify user |

### 7.2 Generated Agent Fault Tolerance

The generated Agent's safety layer handles:

```rust
// safety.rs — the system's fault-tolerance backbone

enum FaultMode {
    LLMTimeout,              // LLM API did not respond
    ToolFailure(String),     // specific tool failed repeatedly
    ContextOverflow,         // context window near limit
    SafetyViolation,         // attempted unsafe action
    UserDisconnect,          // user stopped responding
}

impl SafetyLayer {
    fn handle_fault(&mut self, fault: FaultMode) -> Action {
        match fault {
            FaultMode::LLMTimeout => {
                // Retry once with backoff, then ask user
                if self.retry_count < 1 {
                    self.retry_count += 1;
                    Action::RetryWithBackoff(Duration::from_secs(2))
                } else {
                    Action::AskUser("LLM 超时了，你想怎么做？".into())
                }
            }
            FaultMode::ToolFailure(tool) => {
                // Report tool failure, switch to safe mode if critical
                self.critical_tool_failures.push(tool.clone());
                Action::ReportAndContinue(format!("工具 {} 不可用", tool))
            }
            FaultMode::ContextOverflow => {
                // Summarize and checkpoint before continuing
                Action::CheckpointAndSummarize
            }
            FaultMode::UserDisconnect => {
                // Stop and wait — never auto-proceed without user
                Action::PauseAndWait(Duration::from_secs(300))
            }
        }
    }
}
```

### 7.3 Graceful Degradation

If the LLM fails entirely, the generated Agent degrades predictably:

```
Normal mode:  LLM-based reasoning + tool execution
     │
     ▼ (LLM unavailable)
Degraded mode:  Rule-based fallback (fixed templates, cached responses)
     │
     ▼ (also fails)
Safe mode:  Stop all execution, preserve state, notify user
```

The degradation is **one-way**: you degrade step by step, but only user
intervention restores full capability. This prevents thrashing between modes.

---

## 8. Hierarchical Control

### 8.1 The Two-Layer Hierarchy

**Layer 1: Kyber CLI (timescale: seconds)**

```
Purpose:    Translate user intent into a control system (Agent)
State:      user_intent, risk_profile, tool_set
Output:     kyber.toml + Rust code
Gain:       Low — deliberate, interactive, 6 rounds
```

**Layer 2: Generated Agent (timescale: milliseconds to seconds)**

```
Purpose:    Execute user's goal in the real world
State:      confidence, iteration, context
Output:     commands, keystrokes, file writes
Gain:       Variable — schedule by risk level (0.3-0.9)
```

### 8.2 Timescale Separation

```
Kyber init session:   ~2 minutes total (6 rounds × ~20 seconds each)
Generated Agent step: ~2 seconds per observe-decide-execute cycle

Timescale ratio: 120s / 2s = 60:1

This is well above the 10:1 minimum for stable hierarchical control.
```

### 8.3 Information Flow Between Layers

The information passed from Kyber CLI (upper layer) to the Agent (lower layer)
is the `kyber.toml` — this is the **constraints file**:

```
Upper layer (Kyber) ──────── kyber.toml ────────► Lower layer (Agent)
  (sets constraints:               │
   max_iterations,                 │
   confidence_threshold,           │  Agent operates freely
   require_confirm,                │  within these constraints
   circuit_breaker)                │
                                   ▼
                          At runtime, Agent acts in the world
                                   │
                          Agent collects performance data
                                   │
                          kyber check reads logs,           Lower ──► Upper
                          suggests kyber.toml adjustments   (feedback)
```

### 8.4 Local Absorption

Each layer absorbs its own disturbances:

```
Kyber CLI absorbs:
  - User confusion at a question → clarify, rephrase, provide example
  - File system permission errors → try another path
  - Template compilation errors → retry with fallback template

Generated Agent absorbs:
  - Tool failure → retry with backoff (does not bother user)
  - LLM timeout → retry once (does not escalate)
  - Low confidence → seek one more piece of evidence (not user input yet)

User absorbs:
  - Only what crosses the confidence gate or requires confirmation
  - Only fault modes the Agent cannot handle locally
```

This alignment is deliberate and control-theoretic: **disturbances are handled
at the lowest possible level**.

---

## Appendix: Formal Guarantees of the Design

```
Stability:       Dialogue converges within 6 rounds (Lyapunov energy V < 0.1)
Controllability: Every state dimension has a corresponding question
Observability:   Full state is displayed before generation
Robustness:      CLI handles ambiguous input, user changes mind, file errors
Fault tolerance: All fault modes have defined fallback actions
Timescale sep:   60:1 ratio between layers ≥ 10:1 minimum
Loop gain:       Risk level directly maps to controller gain (0.3-0.9)
**

---

*This design document is itself a control system: its structure constrains
the implementation to produce a predictable, verifiable output — a Kyber Agent
that the user can trust not because they are told to, but because they can
verify it.*
