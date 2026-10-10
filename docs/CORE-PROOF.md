# Integrated Core Preview workload

The `core_preview` example and integration test exercise Core Preview 1 through
`ex`, with expected bytes and pixels calculated separately from the renderer.
This workload completed the first software core milestone on 2026-10-09.

The [example](../crates/ex-api/examples/core_preview.rs) and
[test](../crates/ex-api/tests/core_preview.rs) use the same
[workload](../crates/ex-api/examples/core_preview/workload.rs). The test runs
against every discovered Preview1 adapter and rejects an empty run.

## What it runs

1. Upload padded RGBA8 checker rows and copy them into an atlas rectangle.
2. Transfer seeds and compute four vertices using storage buffers and uniforms.
3. Transition the generated buffer from storage writes to vertex reads.
4. Render a textured indexed quad with depth writes and stencil marks. Reject
   a farther draw and a draw whose stencil comparison fails.
5. Sample that image into two transformed panels, including a vertical flip and
   source-over blending.
6. Read back colour, depth and stencil values, checking every pixel and padding byte.

The valid plan contains 15 commands and 27 combined resource uses, including five
indexed draws across two render scopes. Each successful submission checks 64
generated vertex bytes, 1024 offscreen colour/depth/stencil values, 8192 final
pixels, readback prefixes/suffixes and row padding.

Two submissions reuse the recorded commands with changed upload colours and
fragment parameters. The two panels cover 4608 pixels. Their expected results
use six-pixel checker cells, known panel bounds, a vertical flip and integer
source-over arithmetic. Alpha changes from 1/2 to 1/4.

Local device, image, pipeline, group and seed handles leave scope in `build()`.
Recorded work retains them until execution; inspection and live-input handles
remain in the returned scene. Device colour images deny direct CPU access.

## Failure cases

**Missing dependency.** Omitting the compute-to-vertex barrier returns
`MissingDependency` at command 8 after dispatch 7: `StorageWrite` to `VertexRead`,
`ReadAfterWrite`. Submission rejects the plan before execution. The prefix marker,
geometry, readbacks and attachments remain unchanged.

**Late compute failure.** A finite parameter overflows at invocation `[2,0,0]`
after invocations 0 and 1 produce finite intermediates. Submission returns
`ComputeFailed` at command 7 with `ShaderFailed`. The preceding marker copy
publishes 7, compute output retains its earlier bytes, and later rendering and
readback are skipped. Resetting the parameter to zero allows the same commands
to render the second frame.

## Verification

At the 2026-10-09 checkpoint, all 257 workspace tests and required formatting,
build, Clippy and Rustdoc checks passed. The API had 121 exported types, 229
inherent methods and 34 error types. No core exports were added by this workload.

The 128x64 PNG was checked for chunk CRCs, decompression, dimensions, format and
all 8192 pixels, then inspected visually. Its SHA-256 was
`7027b97e69ccd0a51b9c2b64c69f2cdb63f6b3126101fb4e50521102b7f2b83c`.
Generated images stay under ignored `target/`; the example regenerates them.

```text
cargo test -p ex-api --test core_preview --locked --offline
cargo run -p ex-api --example core_preview --locked --offline
```

The example writes `target/core-preview.png` by default. Its first argument can
select another path. Assertions report mismatches against expected results; EX
misuse returns structured errors. PNG export can fail after rendering succeeds.

The reusable profile cases and feature tests cover additional formats,
instructions, aliases, numeric limits and misuse. This workload checks their
interaction; it is not a hardware conformance or performance test.
