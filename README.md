# Kyber

> **"Every AI agent is a control system."**

A paradigm shift from **Prompt Engineering** (open-loop tinkering) to **Kyber** (closed-loop, stability-guaranteed design).

Inspired by Qian Xuesen's 1954 *Engineering Cybernetics*, this project builds the theoretical and practical framework for designing AI agents using control theory — state-space modeling, stability analysis, observer design, hierarchical control, and online adaptation.

## Core Insight

| Prompt Engineering (Era 1) | Agentic Frameworks (Era 2) | **Kyber (Era 3)** |
|---|---|---|
| Write prompt → run → tweak | Plan → tool call → observe → replan | Model state space → design controller → analyze stability → deploy with guarantees |
| Open-loop | Closed-loop (emergent, not designed) | **Closed-loop (deliberately engineered)** |
| No formal model | Ad-hoc | **State-space modeling + transfer functions** |
| Debug via log reading | Debug via trace inspection | **Stability margin analysis, observer residuals, root locus** |

## Design Document

The full specification is in [`docs/design.md`](docs/design.md).

## Key Concepts

- **Agent Transfer Function**: mapping from user input to agent behavior, characterized by architecture type
- **Stability Margin**: ability to recover from perturbations during agent execution
- **Separation Principle**: observer (state estimation) and controller (action policy) can be designed independently
- **Hierarchical Control**: multi-layer agent orchestration with guaranteed convergence
- **Self-Tuning Agent**: online parameter adaptation with stability guarantees

## Repository Structure

```
├── README.md
├── docs/
│   ├── design.md           # Full design specification
│   ├── 01-state-space.md   # Agent state-space modeling
│   ├── 02-stability.md     # Stability analysis framework
│   ├── 03-hierarchical.md  # Multi-agent hierarchical control
│   ├── 04-adaptation.md    # Self-tuning and online adaptation
│   └── 05-roadmap.md       # Implementation roadmap
└── LICENSE
```

## Status

**Phase 0 — Design & Spec.** Building the theoretical foundation before implementation.
