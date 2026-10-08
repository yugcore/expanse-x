# Expanse-X core

Shared discovery, ownership, descriptor, and recording implementation with no
external dependencies.
The public application entry point is the `ex-api` package, imported as `ex`.

Current behavior:

- `Instance` discovers one software reference `Adapter`.
- `Adapter` creates independent logical `Device` ownership domains.
- `Device` and `Queue` handles retain shared state and expose identity checks.
- `Device::create_buffer` allocates owned reference storage and retains its device.
- `Buffer::write/read` enforce explicit CPU-copy permissions and whole-range bounds.
- `BufferDescriptor` and `BufferUsage` describe bytes and intended roles.
- Validation rejects zero size and missing usage.
- `CommandRecorder` owns debug labels on the CPU.
- Its `finish(self)` returns `DebugMarkerBuffer`, keeping labels distinct from work.
- `Device::create_command_encoder` creates a device-bound recorder for copies.
- `copy_buffer` checks ownership, transfer roles, ranges, and overlap without execution.
- Encoder `finish(self)` produces a `CommandBuffer` retaining its device/resources.

No shaders, draws, queue execution, or submission are implemented.
Names and contracts remain provisional. Marker text never executes graphics work.

From the workspace root:

```text
cargo run -p ex-api --example buffer_roundtrip --locked --offline
cargo run -p ex-api --example record_copy --locked --offline
cargo test --workspace --locked --offline
```

Read the [copy-recording contract](../../docs/COMMANDS.md),
[current contract](../../docs/API-DRAFT.md), and
[architecture](../../docs/ARCHITECTURE.md).
