# Initial prototype

On 2026-10-08, `expansex-core` contained buffer descriptors and a label-only
command recorder. Buffer validation rejected zero size and missing usages.
Finishing a recorder consumed it and returned its recorded labels. Five unit
tests and one compile-fail documentation test covered that behavior.

The prototype had no devices, allocated resources, queues, shaders or rendering.
The first development target was a public Rust entry point with a software
backend, followed by copy execution and an offscreen triangle.

The [API contract](API-DRAFT.md) describes the behavior available at this stage.
