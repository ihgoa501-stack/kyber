# Agent State-Space Modeling

> A practical guide to modeling AI agents as state-space systems.

## Why State-Space?

State-space models describe a system by its internal state variables, inputs, and outputs. For agents, this gives us:

1. A formal language to describe agent behavior
2. Mathematical tools for analysis (controllability, observability, stability)
3. A bridge between agent design and control theory

## General Form

Discrete-time state-space model for a step-based agent:

```
x_{k+1} = A·x_k + B·u_k + w_k
y_k     = C·x_k + v_k
```

## Common Agent State Definitions

### ReAct Agent

```
State:
  - x₁: reasoning_coherence  ∈ [0, 1]   — internal consistency of current reasoning
  - x₂: goal_alignment       ∈ [0, 1]   — alignment with original task objective
  - x₃: confidence           ∈ [0, 1]   — model certainty in current direction
  - x₄: iteration_depth      ∈ ℕ        — number of reasoning steps taken
  - x₅: context_pressure     ∈ [0, 1]   — how much of context window is used

Inputs u:
  - u₁: tool_call_depth      ∈ {0, 1, 2} — how many tools to invoke
  - u₂: reasoning_budget     ∈ ℕ        — max reasoning steps allowed
  - u₃: temperature          ∈ [0, 2]   — LLM sampling temperature

Outputs y:
  - y₁: tool_success_rate    ∈ [0, 1]   — fraction of successful tool calls
  - y₂: output_coherence     ∈ [0, 1]   — coherence of generated text
  - y₃: task_progress        ∈ [0, 1]   — estimated progress toward goal
```

### Reflection Agent

```
State: x₁, x₂, x₃, x₄, x₅ (same as ReAct)
       x₆: reflection_quality  ∈ [0, 1] — quality of self-reflection
       x₇: reflection_count    ∈ ℕ      — number of reflection passes

Extra input u₄: reflection_prompt_detail ∈ {low, medium, high}
Extra output y₄: self_correction_rate    ∈ [0, 1]
```

### RAG Agent

```
State: x₁, x₂, x₃, x₄, x₅ (same as ReAct)
       x₆: retrieval_coverage   ∈ [0, 1] — how much of relevant docs retrieved
       x₇: retrieval_relevance  ∈ [0, 1] — relevance of retrieved chunks

Extra input u₄: chunk_size     ∈ ℕ      — retrieval chunk size
Extra output y₄: answer_groundedness ∈ [0, 1]
```

## Building Your Own State Model

1. **Identify state variables**: What changes between agent steps and matters for behavior?
2. **Identify control inputs**: What knobs does the system designer/controller control?
3. **Identify measured outputs**: What can we observe at runtime?
4. **Estimate transitions**: How does each state variable evolve with each action type?
5. **Verify observability**: Can you reconstruct state from outputs alone?

## Practical Getting Started

For a first approximation, don't worry about exact A, B, C matrices. Instead:

1. Define the state dimensions intuitively
2. Run the agent and collect traces
3. Manually estimate state at each step via expert review
4. Fit a linear model to the trace data
5. Refine iteratively

See the main [design document](design.md) for the full theoretical framework.
