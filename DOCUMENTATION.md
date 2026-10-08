# Expanse-X: project philosophy

Design adopted on 2026-10-08.

This document describes EX's purpose and guiding design commitments. The [roadmap](docs/ROADMAP.md) tracks implementation progress;
[architecture](docs/ARCHITECTURE.md) and topic contracts describe how the current
implementation works. Accepting a philosophy does not make its planned features
implemented.

## Purpose

EX aims to provide explicit graphics control with Rust-based ownership and
validated execution. Applications can let EX manage execution or supply their
own plan, using the same resources and core semantics.

The long-term ambition is Vulkan-level control over meaningful application
decisions: resource allocation and reuse, memory placement, command recording,
queue selection, dependencies, and submission. This is a design target, not a
claim of current feature parity, performance, or control over every internal GPU
scheduling decision. EX defines its own contracts rather than copying Vulkan's
API.

EX should make common state, lifetime, and synchronization mistakes harder to
express, detect invalid work where static proof is impractical, and explain what
valid execution requires. It does not promise to eliminate all programming
errors or infer every shader's data-dependent memory access.

## One core, two control levels

```text
                     Application / game engine
                                |
                               EX
                                |
                    +-----------+-----------+
                    |                       |
              Managed level           Explicit level
                    |                       |
                    +-----------+-----------+
                                |
                             EX Core
                                |
                           EX Runtime
                                |
                         EX Driver Layer
                                |
                   EX reference GPU / future GPU
```

This is the intended logical architecture. It does not mandate a separate crate
for every box or imply that execution and hardware drivers exist today.

**Managed: "Describe what I want rendered."** The application supplies work and
resource-use information. EX derives necessary dependencies and chooses a valid
execution plan within the application's requirements. Managed defaults should
make ordinary work straightforward; their execution decisions and costs should
be inspectable.

**Explicit: "Let me decide how that work executes."** The application supplies
the plan and dependencies for the controls EX exposes. EX validates that plan
against the execution contract. Invalid plans produce understandable errors;
EX must not silently repair missing dependencies by substituting a different
schedule. Correctness-required behavior documented in the contract is distinct
from a hidden repair.

Both levels must share resource identity, device ownership, command meaning,
shader and pipeline contracts, and completion semantics. Applications should be
able to move individual workloads between levels without replacing their
resources. Mixing levels requires explicit, documented boundary information:
resource uses, dependencies, and completion. Interoperability is a commitment;
the exact public methods and boundary representation remain future design work.

The managed level is built on the same explicit core and execution rules. It
must not become a separate renderer with different resource or safety semantics.
Explicit control means choosing a valid plan, not disabling safety checks.

## Safety responsibilities

| Mechanism | Intended responsibility | Boundary |
| --- | --- | --- |
| Rust types and ownership | Express legal states and prevent misuse such as using a consumed encoder | The compiler does not automatically model GPU execution or prove all resource dependencies |
| Runtime validation | Check dynamic device ownership, ranges, usages, state compatibility, and declared dependencies | Validation needs sufficient resource-use information; unavailable information must have a defined contract |
| Execution lifetime management | Retain resources used by recorded/submitted work and coordinate CPU access with pending execution | Keeping storage alive does not establish ordering, visibility, or completion |

The safe public interface must uphold its documented safety guarantees in both
levels and in all build profiles. Optional expensive diagnostics may have a
separate policy, but turning them off must not remove mechanisms required for
safe execution. Any future unsafe platform or interoperability boundary must be
isolated, justified, and document the obligations it transfers to the caller.

Failures should identify the operation, relevant resource, violated requirement,
and useful next action, with machine-readable errors where appropriate. EX must
define failure effects, including whether earlier work may have executed. Safety
does not imply correct application logic, an intended image, deadlock freedom,
or identical pixels on every future hardware target.

## Example: upload followed by drawing

This example describes the intended design, not available API methods.

1. An upload writes a vertex buffer.
2. A draw reads that buffer.
3. The draw requires the upload's writes to be visible before it reads them.

At the managed level, EX uses declared resource accesses to establish this
dependency and exposes the resulting plan for inspection. At the explicit level,
the application supplies the dependency and EX checks it. Both must retain the
buffer for the work's lifetime and follow the same visibility rules.

The distinction is who chooses the plan. Neither level makes an invalid
read/write conflict acceptable. Shader-internal synchronization and arbitrary
memory access require their own contracts as programmability grows.

## Independent execution and useful differentiation

EX is an independent Rust graphics API with its own runtime, commands, resource
semantics, shader direction, and execution backend. Vulkan, Direct3D, Metal,
OpenGL, and wgpu may be studied as references; they do not perform EX rendering.
The first execution target is EX's own CPU software/reference GPU. Direct
hardware-driver work remains deferred until reference semantics are useful and
testable, with a viable target and operating-system interfaces.

EX's proposed value is a coherent combination of explicit control, managed
convenience, understandable diagnostics, and inspectable reference execution.
Rust, automatic dependency handling, and two control levels alone are not claims
of a feature unique to EX. Performance and usability benefits require examples
and measurements rather than assertions.

## Implementation order and current reality

Build the explicit core first and prove its rules with small working operations.
Then add managed conveniences over those proven contracts when a real example
justifies them. Do not create two API trees, empty backend crates, or a complete
theoretical scheduler before work can execute.

Today EX implements software discovery, logical device/queue ownership, owned
buffers, checked synchronous CPU access, and label-only command recording.
Executable copies, submission, rendering, shaders, managed planning, GPU hazard
validation, and hardware drivers are not implemented. The next step remains
recorded buffer copies and queue submission, as defined by the roadmap.

## Design commitments

Future designs and roadmap changes must conform to these commitments. Replacing
the independent execution model, splitting the levels into incompatible systems,
or weakening the safe-interface guarantees requires an explicit design
revision of this document, with the reason recorded here and related documents
updated together.

Public names, data structures, module boundaries, and individual algorithms can
evolve through normal implementation work within these commitments. "Locked"
means a stable design direction, not a frozen API or a claim of completeness.

Decision record: 2026-10-08 — accepted one core with managed and explicit control
levels, Rust-based ownership, validated execution, explicit core first, and EX's
own reference execution before hardware drivers.
