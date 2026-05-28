# Kyber CLI

Control-engineered Agent scaffolder. Generates Rust agent projects
with built-in trust guarantees — confidence gates, safety layers,
audit logs, and pre/post execution verification.

## Usage

```bash
# Create a new agent project
kyber init my-agent

# Validate existing configuration
cd my-agent && kyber check
```

## Generated Structure

```
my-agent/
├── kyber.toml         # Control system configuration
├── Cargo.toml
└── src/
    ├── main.rs         # Control loop with trust layers
    ├── control/
    │   ├── safety.rs   # Circuit breaker, confirm, audit
    │   ├── observer.rs # Confidence estimation
    │   └── controller.rs # Decision logic
    └── tools/
        └── mod.rs      # Tool registry
```

## Architectures

- **react**: Standard observe → decide → execute → verify loop
- **deep-verify**: Adds pre-execution simulation and post-execution verification
