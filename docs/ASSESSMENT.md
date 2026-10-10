# Initial prototype

On 2026-10-08, `expansex-core` contained buffer descriptors and a label-only
command recorder. Buffer validation rejected zero size and missing usages.
Finishing a recorder consumed it and returned its recorded labels. Five unit
tests and one compile-fail documentation test covered that behavior.

There were no devices, allocated resources, queues, shaders or rendering yet.
The first development target was a public Rust entry point with a software
backend, followed by copy execution and an offscreen triangle.

That prototype remains available through the
[`api_draft` example](../crates/ex-api/examples/api_draft.rs). Its labels are
separate from the device-bound command buffers used for execution.

For current behavior, use the [API catalog](../api.md) and
[Core Preview 1 profile](CORE-PROFILE.md). The [roadmap](ROADMAP.md) tracks
remaining work.
