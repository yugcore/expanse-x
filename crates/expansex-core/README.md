# Expanse-X core

Shared discovery, ownership, descriptor, and recording implementation with no
external dependencies.
The public application entry point is the `ex-api` package, imported as `ex`.

Current behavior:

- `Instance` discovers one software reference `Adapter`.
- `Adapter` creates independent logical `Device` ownership domains.
- `Device` and `Queue` handles retain shared state and expose identity checks.
- `BufferDescriptor` and `BufferUsage` describe bytes and intended roles.
- Validation rejects zero size and missing usage.
- `CommandRecorder` owns debug labels on the CPU.
- `finish(self)` consumes the recorder and returns an immutable `CommandBuffer`.

No buffer storage, shaders, draws, queue execution, or submission are implemented.
Names and contracts remain provisional. Marker text never executes graphics work.

From the workspace root:

```text
cargo run -p ex-api --example discover --locked --offline
cargo test --workspace --locked --offline
```

Read the [current contract](../../docs/API-DRAFT.md) and
[architecture](../../docs/ARCHITECTURE.md).
