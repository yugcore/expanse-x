# ExpanseX core: first API draft

This is a small executable API design prototype with no external dependencies.

It currently provides:

- `BufferDescriptor` and `BufferUsage` to describe requested storage and roles.
- Descriptor validation rejecting zero size and missing usage.
- `CommandRecorder` that records debug labels on the CPU.
- `finish(self)` that consumes the recorder and returns a `CommandBuffer`.

No GPU buffers, devices, queues, shaders, draw commands, or submission are implemented. Names and contracts remain provisional. Markers naming future operations do not perform those operations.

From the workspace root:

```text
cargo run -p expansex-core --example api_draft
cargo test -p expansex-core
```

Use package-specific commands from the repository root.

The [design contract](../../docs/API-DRAFT.md) records current behavior and open questions.
