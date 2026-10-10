# Core Preview milestone

Core Preview 1 was completed on 2026-10-09. It defines the first supported
software profile; the public API is still provisional. SDK development builds
on this core and is now in progress.

## Scope

The core owns resources, shaders, pipelines, commands, dependencies and execution.
It defines validation, failure behavior and resource lifetimes. The SDK provides
the project templates, shader tools and build workflow that applications use to
work with it.

The first core milestone covers explicit, synchronous execution on the CPU.
Hardware support, managed planning and asynchronous work need separate contracts.
The shared managed/explicit design is described in [DOCUMENTATION.md](../DOCUMENTATION.md).

## Completed requirements

| Requirement | Implementation and tests |
| --- | --- |
| Resources and pipelines | Owned storage, programmable graphics, compute and validation through `ex`; see [API-DRAFT.md](API-DRAFT.md) |
| Resource dependencies | Whole-resource access roles and explicit dependency validation; see [DEPENDENCIES.md](DEPENDENCIES.md) |
| Shader interfaces | Float4 instructions, reusable binding layouts/groups, uniforms, textures and compute storage; see [BINDINGS.md](BINDINGS.md) |
| Image flow | Padded transfers, rectangular copies and shared render-to-texture attachments; see [TEXTURE-FLOW.md](TEXTURE-FLOW.md) |
| Execution profile | Device-owned executor, runtime sequencing and error handling, limits and reusable adapter tests; see [CORE-PROFILE.md](CORE-PROFILE.md) |
| Integrated workload | Shared example/test covering upload, compute, indexed two-pass rendering, readback and failure/reuse cases; see [CORE-PROOF.md](CORE-PROOF.md) |

These requirements are checked by the public examples, profile tests and feature
tests. Supported operations and limits are documented; richer features remain
on the [roadmap](ROADMAP.md).

## Work after the milestone

The SDK now checks setup, creates Rust/GLSL projects, compiles/imports a limited
shader subset and reports reflection. A Windows x64 preview package is available.
Safe shader reload is next; asset/window integration and broader tooling follow.
See [SDK.md](SDK.md) for the workflow.

Asynchronous completion, multiple queues, indirect operations, advanced memory
placement and presentation are further core work. Managed planning will share
the explicit core's resources and execution rules. Hardware drivers are deferred;
Witness and Proof remain proposed experiments.
