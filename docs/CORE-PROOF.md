# Integrated Core Preview proof

The maintained
public workload must exercise Preview1 through `ex`, with independent expected
bytes/pixels and the existing reusable profile/topic suites. Evidence is recorded
after checks succeed. This closes the bounded explicit software core milestone;
it does not close the broader Phase 4 or stable v1 roadmap.

## Workload and acceptance

The `core_preview` example and integration test will execute the same shared
workload, avoiding a separate demonstration that CI merely compiles. The test
will run against every discovered Preview1 adapter and reject an empty run.

The workload will upload padded RGBA8 checker rows, copy them into an atlas
rectangle, transfer seeds, and compute four vertices using storage and uniforms.
Explicit transitions will make the generated vertex allocation usable by an
indexed draw. An offscreen pass will render a textured quad with depth writes
and stencil marks, reject a farther draw, and reject a mismatching stencil draw.
A second pass will sample the rendered image into two transformed panels,
including a vertical flip and source-over blending. Padded buffer readback will
verify all final/offscreen pixels and untouched row padding. Depth/stencil reads
will verify all attachment values. Device colour images will deny CPU access.

Two successful submissions of unchanged recorded commands will change upload
colours and fragment parameters. Expected panel coverage and checker coordinates
will be calculated analytically, independently of the rasterizer. A PNG export
will come from the verified final readback, with independent CRC/decompression/
pixel verification and visual inspection before declaring this gate complete.

Two negative paths are required in this same retained workload:

- Omit the compute-to-vertex dependency in a separately recorded plan. Submission
  must reject the whole plan before a prefix marker copy or any output changes.
- Supply a finite compute parameter that overflows only after initial invocations
  succeed. A preceding marker copy must complete, all compute output bytes must
  remain unchanged, and later render/readback/attachment effects must be skipped.
  Restore valid data and submit the original successful plan again.

Local device, image, pipeline, group and seed handles will leave scope before
execution; inspection and live-input handles remain in the returned Scene. Recorded work
retains its device, pipelines, groups and resources; synchronous return determines
when readback can be checked. Expected failures use existing structured errors.

## Evidence boundary

This workload integrates the core categories; the nine reusable profile cases
and detailed topic suites remain responsible for broader instruction, format,
alias, numeric and misuse coverage. One image cannot prove every enum variant or
all programs. No new exported API, graphics dependency, hardware target, managed
planner, shader importer, SDK or async execution is required for this gate.
Project checks, the API inventory and feature contracts must agree with the
implemented profile.

## Tests and examples

Completed and verified on 2026-10-09. The [example](../crates/ex-api/examples/core_preview.rs)
and [integration test](../crates/ex-api/tests/core_preview.rs) use the same
[public workload](../crates/ex-api/examples/core_preview/workload.rs). The test
asserts at least one Preview1 adapter was exercised. No core exports changed.

The valid plan contains 15 commands and 27 combined resource uses, including
five indexed draws across two render scopes. Each of two successful submissions
checks 64 generated vertex bytes, all 1024 offscreen pixels, all 1024 depth and
stencil values, all 8192 final pixels, both readback prefixes/suffixes and every
row's padding. The two panels contain 4608 pixels; their independent oracle uses
six-pixel checker cells, known panel bounds, a vertical flip and integer
source-over colour arithmetic. Alpha changes from 1/2 to 1/4 on unchanged commands.

The omitted barrier yields MissingDependency at command 8 after dispatch 7,
StorageWrite to VertexRead, ReadAfterWrite. Snapshots confirm that prefix marker,
geometry, readbacks and attachments are untouched. The runtime failure is
ComputeFailed at command 7, ShaderFailed at invocation [2,0,0]; invocations 0 and
1 have finite intermediate results. The marker copy publishes 7, the geometry
retains its earlier bytes, readbacks retain their pixels/padding, and all depth/
stencil values remain unchanged. Restoring the parameter to zero allows the same
recorded commands to produce the second verified frame. Construction handles
leave scope in build() before these submissions; retained work keeps them alive.

Independent PNG verification checks every chunk CRC, zlib decompression, IHDR
dimensions/format and all 8192 pixels. The 128x64 image was visually inspected.
SHA256: `7027b97e69ccd0a51b9c2b64c69f2cdb63f6b3126101fb4e50521102b7f2b83c`.
Generated images remain in ignored target/; the maintained source regenerates them.

All 257 workspace tests pass, including this test, the nine profile cases and
executable documentation. Required formatting, all-target build, warnings-denied
linting and API documentation checks pass. The API inventory remains 121 exported
types, 229 inherent methods and 34 error types. Feature contracts are updated. The bounded Core Preview completion gate is met.

```text
cargo test -p ex-api --test core_preview --locked --offline
cargo run -p ex-api --example core_preview --locked --offline
```

The example writes `target/core-preview.png` by default; its first argument can
select another output path. Assertions intentionally report oracle regressions;
expected EX misuse is matched as a structured Result error. An image-export I/O
failure is separate from completed EX execution. Comprehensive allocation-failure
recovery, performance benchmarking, external certification and full GPU coverage
are not established by this milestone. SDK development is the next milestone.
