# Expanse-X design

Design adopted on 2026-10-08. The [roadmap](docs/ROADMAP.md) tracks progress;
[architecture](docs/ARCHITECTURE.md) describes the implementation.

## Purpose

EX aims to give applications explicit graphics control with Rust ownership and
runtime validation. Applications should be able to choose how much execution
planning they handle while using the same resources and core rules.

The long-term target is Vulkan-level control over allocation, memory placement,
recording, queues, dependencies and submission. Today's software profile covers
only part of that scope. Rust types can prevent some state and lifetime mistakes;
dynamic ranges, resource ownership and dependencies still need runtime checks.

## One core, two control levels

```text
                   Application
                        |
                       EX
                 /            \
          Managed API      Explicit API
                 \            /
                     EX Core
                        |
                    EX Runtime
                        |
                Execution backend
                        |
             CPU today / GPU later
```

**Managed: "Just get it rendered."** The application supplies work and resource
uses. EX derives dependencies and chooses a valid execution plan within the
application's requirements. The chosen plan and its costs should be inspectable.

**Explicit: "Give me control over how it gets rendered."** The application supplies
the plan and dependencies. EX checks them against its execution rules and reports
invalid work. It must not silently repair a missing dependency by choosing a
different schedule.

Both levels share resource identity, device ownership, command meaning, shader
and pipeline behavior, and completion rules. Moving a workload between them
should not require replacing its resources. Mixed workloads must describe their
resource uses, dependencies and completion at the boundary.

The managed layer will build on the explicit core. Its public API and the mixed
workload interface are still to be designed. The explicit software core is
implemented; managed planning is future work.

## Safety and failure

| Mechanism | Responsibility |
| --- | --- |
| Rust types and ownership | Express valid states and prevent use of consumed values, such as a finished encoder |
| Runtime validation | Check device ownership, ranges, usages, compatible state and declared dependencies |
| Resource retention | Keep recorded/submitted resources alive and coordinate CPU access with execution |

Retention alone does not establish ordering or completion. Rust ownership alone
does not prove shader memory access or GPU synchronization. Each feature needs
a contract for the information validation requires and the accesses it supports.

The safe interface must uphold its guarantees in both control levels and all
build profiles. Optional diagnostics may be expensive, but disabling them must
not remove checks needed for safe execution. Any future unsafe interoperability
boundary must document the caller's obligations.

Errors should identify the operation, resource, failed requirement and a useful
next action. Failure behavior must state whether earlier commands ran and what
state remains available. Safety checks cannot determine whether an image matches
the application's intent.

For example, an upload writes a vertex buffer and a later draw reads it. A managed
planner would derive the dependency; an explicit application declares it. Both
must keep the buffer alive and make the upload visible before the read.

## Execution

EX owns its resource model, commands, shader execution and rendering backend.
The first backend runs on the CPU. Other graphics APIs are references for design,
not execution dependencies. Hardware work follows a tested reference profile and
a viable target driver interface.

Core Preview 1 is complete for synchronous software execution. The first SDK
preview adds project generation, bounded GLSL/SPIR-V import and shader builds.
See [CORE-PROFILE.md](docs/CORE-PROFILE.md) for current limits and
[SDK.md](docs/SDK.md) for the developer workflow. A stable v1 API remains a later
goal. New core features should follow workloads that need them.

## EX Witness: experimental differentiation

Witness is a proposed debugging workflow: select a failure, explain the relevant
work, replay it and export a smaller runnable reproduction. It is not implemented.

Start with a buffer-copy error or an application-provided readback assertion:

1. Define the condition the reproduction must preserve.
2. Capture commands, resource descriptions, initial contents and dependencies.
3. Replay the case through the reference backend without the original application.
4. Remove unrelated work, revalidating each candidate against the same condition.
5. Export the reproduction with its EX version and environment requirements.

A repaired reproduction should become a regression test. Later experiments could
explain a pixel's depth-test result or trace shader values. A wrong-looking image
alone cannot tell EX what the developer intended, and reduction cannot guarantee
the smallest possible case.

Capture must reproduce validation failures without executing rejected work.
Tracing and repeated replay have memory and execution costs, so developers enable
them explicitly. Hardware timing bugs may not reproduce on the software backend.
Managed and explicit work use the same capture and execution rules.

[RenderDoc](https://github.com/baldurk/renderdoc/blob/v1.x/docs/how/how_inspect_pixel.rst)
and [Nsight](https://docs.nvidia.com/nsight-graphics/UserGuide/frame-debugger-overview.html)
already provide pixel history and shader debugging. The EX experiment is to
connect explanations, reduction and runnable tests within its own execution model.

## EX Proof: experimental execution certificates

Proof is a proposed certificate for specific resource-access obligations in an
execution plan. It is not implemented.

Start with buffer copies and their ranges and dependencies. A certificate would
record the model version, checked properties, assumptions, exact plan and relevant
resource parameters. A small separate checker would verify it without executing
copies or trusting the planner's verdict.

For example, when copy A writes bytes that copy B reads, the certificate could
identify the dependency that orders those accesses. Managed work would generate
a plan and certificate; explicit work would be checked without changing its plan.

The first experiment must reject missing dependencies, corrupted certificates
and certificates reused against changed plans or resources. Unsupported scope
must have its own result. Output should be structured data, with console reports
or editor views available on request. Normal rendering should remain quiet.

A certificate covers only the stated model and assumptions. Shader-dependent
accesses, aliasing, CPU concurrency and external inputs need to be modeled or
excluded. The checker is trusted code; runtime execution must still obey the
checked plan. Changes to relevant inputs require rechecking. This does not prove
hardware correctness, intended pixels or optimal performance.

[WEFT](https://research.nvidia.com/publication/2015-06_verification-producer-consumer-synchronization-gpu-programs)
and [AutoSync](https://cseweb.ucsd.edu/~npolikarpova/publications/fmcad18.pdf)
provide prior work on GPU synchronization verification and synthesis. EX's
experiment would combine checkable certificates with readable explanations.
A failed check could later supply a failure condition for Witness; the two
experiments can be developed separately.

## Design commitments

The independent execution model, shared managed/explicit core, explicit
application control and safe-interface guarantees are the project's agreed
direction. Changes to them require an explicit design revision. Public names,
formats, crate boundaries and implementation details may evolve within those
commitments.

Witness and Proof were added as experiments on 2026-10-08. Their names do not
freeze an API or promise a release. Usability, performance and differentiation
need working examples and measurements.
