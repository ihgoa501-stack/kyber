# Hierarchical Control for Multi-Agent Systems

> Structured orchestration of multiple agents using layered control theory.

## The Principle

A complex task should be decomposed into layers of control, where each layer:
1. Is a complete control system in its own right
2. Operates at a distinct timescale
3. Communicates with adjacent layers via aggregated information
4. Absorbs disturbances before they propagate

## Three-Layer Architecture

### Layer 3: Strategic (Orchestrator)

```
Purpose:      Goal decomposition and agent dispatch
Timescale:    Seconds to minutes
Gain:         Low (conservative, aggregated oversight)
State:        Overall task progress, agent availability, quality metrics
Input:        User request (natural language)
Output:       Task specifications (structured contracts)
Control:      Plan → delegate → monitor → replan
```

**Contract format** passed from L3 to L2:

```yaml
task_contract:
  id: "task-001"
  goal: "Analyze the database schema and generate migration plan"
  constraints:
    max_agents: 3
    max_tokens_per_agent: 4000
    deadline_seconds: 120
  output_spec:
    format: "migration_plan"
    required_fields: ["changes", "risks", "rollback"]
  success_criteria:
    - "All schema changes identified"
    - "Migration script generated"
    - "Rollback plan included"
```

### Layer 2: Tactical (Specialist Agents)

```
Purpose:      Sub-task execution, tool orchestration
Timescale:    Hundreds of milliseconds to seconds
Gain:         Medium
State:        Sub-task progress, tool availability, intermediate results
Input:        Task contracts (from L3)
Output:       Completed results + confidence scores
Control:      Reason → tool call → observe → iterate
```

### Layer 1: Operational (Tools)

```
Purpose:      Atomic operations
Timescale:    Milliseconds
Gain:         High (fast, deterministic)
Input:        Command + parameters
Output:       Result + status code
Control:      Immediate execution with retry logic
```

## Design Rules

### Timescale Separation
The ratio between layer timescales must be ≥ 10:1. If L1 operates at 100ms, L2 must operate at ≥ 1s, and L3 at ≥ 10s.

Violation example: If L3 replans every time a single tool call fails, L3 and L2 operate at the same timescale → thrashing.

### Gain Hierarchy
Control gain decreases as you go up the hierarchy:
- L1: Gain = 0.8–1.0 (react immediately, retry aggressively)
- L2: Gain = 0.3–0.6 (measured response, limited retries)  
- L3: Gain = 0.1–0.3 (deliberate, require multiple signals to trigger replan)

### Information Aggregation
L2 → L3 communication must be summaries, not raw data:

```python
# Wrong: raw signal propagation
L2_to_L3_bad = {
    "tool_outputs": [...],  # raw output from every call
}

# Right: aggregated state
L2_to_L3_good = {
    "task_status": "in_progress",
    "progress_pct": 0.65,
    "confidence": 0.82,
    "bottleneck": "database_connection",  # abstracted
    "escalated_issues": ["permission_denied_on_table_X"],
}
```

### Local Absorption
A disturbance at L1 should never reach L3. If a tool call fails:
- L1: Retry with backoff (handles transient failures)
- L2: Only notify L3 if retry budget exhausted or the tool is essential
- L3: Replan only if L2 reports an unrecoverable error

## Stability of Hierarchical Systems

A hierarchical agent system is stable if:

1. Each layer is individually stable (its internal controller converges)
2. The coupling between layers is contractive — perturbations passing upward or downward decay by at least a factor of 0.5 per layer

### Verification Protocol

```python
def test_hierarchical_stability(system, test_scenarios):
    for scenario in test_scenarios:
        # Inject perturbation at each layer
        for layer in [L1, L2, L3]:
            result = system.run_with_perturbation(
                layer=layer,
                perturbation=scenario.perturbation,
                measure="propagation_upward"
            )
            assert result.attenuation_per_layer > 2.0, \
                f"Perturbation propagates: {result}"
```

## Implementation Sketch

```python
class HierarchicalAgentSystem:
    def __init__(self):
        self.strategic = StrategicController(
            timescale_ms=10000,
            gain=0.2
        )
        self.tactical_pool = [
            TacticalAgent(id=f"agent-{i}", timescale_ms=1000, gain=0.5)
            for i in range(3)
        ]
        self.tool_layer = ToolRegistry()
    
    async def run(self, user_request):
        # L3: Plan
        plan = await self.strategic.plan(user_request)
        
        for task in plan.tasks:
            # L3 → L2: dispatch contract
            agent = self.select_agent(task)
            task_result = await agent.execute(
                task, 
                self.tool_layer,
                on_progress=lambda p: self.strategic.monitor(p)
            )
            
            # Check if L3 needs to replan
            if self.strategic.should_replan(task_result):
                plan = await self.strategic.replan(task_result, plan)
        
        return plan.aggregate_results()
```
