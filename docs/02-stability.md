# Stability Analysis for AI Agents

> How to analyze whether an agent will converge, oscillate, or diverge.

## Core Concept

An agent is **stable** if small perturbations during execution produce small effects that decay over time. An agent is **unstable** if perturbations grow, causing the agent to diverge from its intended behavior.

## Stability Testing Protocol

### Step 1: Inject a Known Perturbation

At step k of the agent's execution, inject a controlled error:

```python
def test_stability_margin(agent, task, perturbation_fn, n_steps=10):
    """
    Run agent on task, inject perturbation at step 3.
    Measure how quickly behavior converges back to baseline.
    """
    baseline = run_agent(agent, task)
    perturbed = run_agent(agent, task, on_step=3, apply=perturbation_fn)
    
    deviation_series = []
    for i in range(n_steps):
        deviation = measure_deviation(baseline[i], perturbed[i])
        deviation_series.append(deviation)
    
    # Fit decay rate
    decay_rate = fit_exponential_decay(deviation_series)
    
    return {
        "margin": -decay_rate,  # positive = stable, negative = unstable
        "settling_time": first_index_below(deviation_series, 0.05 * deviation_series[0]),
        "overshoot": max(deviation_series) / deviation_series[0],
    }
```

### Step 2: Compute the Lyapunov Energy

```python
def lyapunov_energy(state):
    """Energy function that should decrease for a stable agent."""
    return (
        w1 * (1 - state.reasoning_coherence) ** 2 +
        w2 * (1 - state.goal_alignment) ** 2 +
        w3 * state.iteration_depth / MAX_ITERATIONS +
        w4 * state.context_pressure
    )
```

**Stability condition**: V(x_{k+1}) ≤ V(x_k) for all k after the perturbation.

### Step 3: Interpret Results

| Decay rate | Settling time | Overshoot | Verdict |
|---|---|---|---|
| > 0.1 | < 3 steps | < 10% | Stable |
| > 0.01 | < 10 steps | < 30% | Marginally stable |
| ~ 0 | > 20 steps | — | Low stability margin |
| < 0 | Never settles | > 100% | **Unstable** |

## Common Agent Instability Patterns

### Oscillation (Hunting)
The agent alternates between two strategies without converging.

```yaml
symptoms:
  - agent calls tool A, then tool B, then tool A, then tool B...
  - never making progress between cycles
  - iteration count grows without progress
diagnosis:
  reason: "Coupled control loops at same timescale"
  fix: "Add damping (cooldown period) or decouple timescales"
```

### Divergence (Drift)
The agent's reasoning moves progressively away from the goal.

```yaml
symptoms:
  - reasoning becomes increasingly tangential
  - tool calls become less relevant over time
  - final output bears little relation to task
diagnosis:
  reason: "No (or insufficient) feedback on goal alignment"
  fix: "Add periodic goal-checking step in the reasoning loop"
```

### Limit Cycle (Stuck)
The agent reaches a stable but wrong conclusion.

```yaml
symptoms:
  - agent confidently produces the same wrong answer
  - refuses to reconsider even with contradictory evidence
  - output is internally consistent but externally wrong
diagnosis:
  reason: "Attractor in state space corresponding to a local minimum"
  fix: "Add exploration mechanism (temperature boost, external validation)"
```

## Designing for Stability

### Design Rule 1: Close Every Loop
Every component that influences agent behavior must have a corresponding feedback signal. If the agent can do something it cannot observe the effect of, that's an open-loop path — and open-loop paths always drift.

### Design Rule 2: Low-Pass Filter Your Signals
Raw tool outputs and logprobs are noisy. Apply a moving average before using them for decisions:

```python
confidence = 0.3 * current_logprob + 0.7 * prev_confidence
```

### Design Rule 3: Add Damping
Never let the agent make decisions based on a single observation. Require N consecutive signals before changing strategy (dead zone / hysteresis).

### Design Rule 4: Cap Loop Gain
If the agent's behavior changes too rapidly in response to input, it will oscillate. Introduce observation windows that limit reaction speed.

## Measurement Framework

```python
# Stability measurement test suite
stability_tests = [
    PerturbationTest(
        name="wrong_tool_output",
        inject=lambda state: {**state, 'tool_result': 'unexpected_error'},
        measure=lambda trace: trace.variance('strategy'),
    ),
    PerturbationTest(
        name="ambiguous_input",
        inject=lambda state: {**state, 'user_input': '[REDACTED]'},
        measure=lambda trace: trace.variance('confidence'),
    ),
    StressTest(
        name="high_concurrency",
        inject=lambda: simulate_concurrent_requests(rate=100),
        measure=lambda results: results.p99_latency_ratio,
    ),
]
```

Application: run this suite before deploying any agent to production.
