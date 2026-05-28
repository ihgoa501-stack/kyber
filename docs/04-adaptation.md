# Self-Tuning Agents — Online Adaptation

> How agents can adapt their own parameters at runtime with stability guarantees.

## Motivation

A fixed-parameter agent is brittle. The optimal temperature, reasoning depth, and tool selection strategy depend on:
- Task difficulty and domain
- Current LLM model quality and latency
- Environmental conditions (load, API availability)
- User expectations and feedback patterns

A self-tuning agent continuously adjusts its parameters to remain optimal across changing conditions.

## The Adaptation Loop

```
                     ┌─────────────────┐
                     │  Reference Model │
                     │  (ideal behavior)│
                     └────────┬────────┘
                              │ reference output
                              ▼
┌──────────┐   measured   ┌──────────────┐   error   ┌───────────┐
│  Agent   │─────────────►│  Performance  │─────────►│ Parameter │
│  System  │              │  Monitor      │          │ Adaptor   │
│          │◄─────────────┤               │◄─────────┤           │
└──────────┘   updated    └──────────────┘  adapt.   └───────────┘
               parameters               law params
```

## Adaptation Laws

### Temperature Adaptation

```python
def adapt_temperature(
    current_temp: float,
    logprob_history: list[float],
    entropy_history: list[float],
    gt: float  # adaptation gain
) -> float:
    """
    Decrease temperature when confidence is high and stable.
    Increase when confidence is low or oscillating.
    """
    mean_logprob = np.mean(logprob_history[-5:])
    logprob_std = np.std(logprob_history[-5:])
    entropy_trend = np.polyfit(range(len(entropy_history[-5:])), entropy_history[-5:], 1)[0]
    
    # Error signal: deviation from ideal confidence zone
    error = (0.7 - mean_logprob)  # target: 0.7 average logprob
    
    # Adaptation with damping
    damping = max(0.1, 1.0 - logprob_std * 2)
    delta = -gt * error * damping
    
    # Penalize rising entropy (uncertainty trend)
    if entropy_trend > 0.05:
        delta += gt * 0.1  # push temperature up to explore
    
    new_temp = np.clip(current_temp + delta, 0.1, 2.0)
    
    # Anti-windup: if at boundary, stop integrating
    if (new_temp <= 0.1 and delta < 0) or (new_temp >= 2.0 and delta > 0):
        return current_temp
    
    return new_temp
```

### Tool Call Frequency Adaptation

```python
def adapt_tool_frequency(
    current_frequency: float,
    tool_success_rate: float,
    task_complexity_estimate: float,
    gf: float
) -> float:
    """
    Increase tool usage when tools are helpful.
    Decrease when tools are failing or unnecessary.
    """
    # Error: performance gap
    performance = tool_success_rate * task_complexity_estimate
    error = 0.8 - performance  # target: 80% effective tool usage
    
    # Adaptation
    delta = -gf * error
    new_frequency = np.clip(current_frequency + delta, 0.1, 1.0)
    
    return new_frequency
```

### Reasoning Depth Adaptation

```python
def adapt_reasoning_depth(
    current_depth: int,
    task_difficulty: float,
    iteration_efficiency: float,
    gd: float
) -> int:
    """
    Increase reasoning depth for hard tasks where it helps.
    Decrease for simple tasks where it's wasteful.
    """
    # Reference: ideal depth based on difficulty
    reference_depth = int(task_difficulty * 5) + 1
    
    # Error
    error = reference_depth - current_depth
    
    # Adapt with efficiency penalty
    if iteration_efficiency < 0.3 and current_depth > 1:
        error -= 2  # penalize deep reasoning when inefficient
    
    delta = int(np.round(gd * error))
    return np.clip(current_depth + delta, 1, 10)
```

## Stability Guarantee for Adaptation

The key theorem from Model Reference Adaptive Control:

> If the adaptation gain γ is sufficiently small and the reference model is stable, the adapted system remains stable.

**Practical rule**: Start with γ = 0.01 and double until the system oscillates, then halve.

```python
# Gain scheduling: higher when far from optimum, lower when near
def compute_adaptation_gain(error_magnitude: float) -> float:
    if error_magnitude > 0.5:
        return 0.05  # fast convergence far from target
    elif error_magnitude > 0.1:
        return 0.02  # moderate
    else:
        return 0.005  # fine-tuning near target (anti-oscillation)
```

## Monitoring Adaptation Health

```python
class AdaptationMonitor:
    def __init__(self):
        self.parameter_history = []
        self.convergence_metrics = {}
    
    def record(self, params, performance):
        self.parameter_history.append({
            "params": params,
            "performance": performance,
            "timestamp": time.time()
        })
        
        # Detect oscillation
        if len(self.parameter_history) > 10:
            temp_trace = [h["params"].temperature for h in self.parameter_history[-10:]]
            self.convergence_metrics["temp_oscillation"] = np.std(temp_trace)
            
            if self.convergence_metrics["temp_oscillation"] > 0.3:
                logger.warning("Temperature oscillation detected — reducing gain")
                return {"action": "reduce_gain", "factor": 0.5}
        
        return {"action": "continue"}
```

## Implementation Architecture

```python
class SelfTuningAgent:
    def __init__(self, initial_params: AgentParams):
        self.params = initial_params
        self.observer = AgentObserver()
        self.adaptor = ParameterAdaptor(
            adaptation_laws=[
                TemperatureAdaptation(gamma=0.01),
                ToolFrequencyAdaptation(gamma=0.01),
                ReasoningDepthAdaptation(gamma=0.1),
            ],
            stability_monitor=StabilityMonitor()
        )
        self.history = PerformanceHistory(window=20)
    
    async def step(self, task, environment):
        # Execute with current parameters
        result = await self.execute_with_params(task, environment)
        
        # Measure performance
        signals = self.observer.observe(result)
        self.history.record(signals)
        
        # Adapt parameters (if enough data)
        if self.history.ready():
            for law in self.adaptor.adaptation_laws:
                new_val = law.adapt(
                    self.params.get(law.param_name),
                    self.history,
                    law.gamma
                )
                self.params.set(law.param_name, new_val)
        
        # Stability check
        status = self.adaptor.stability_monitor.check(
            self.params, self.history
        )
        if status.alert:
            self.params = status.safe_fallback_params
        
        return result, self.params
```
