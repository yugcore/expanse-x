# EX current API contract

Status: Phase 0 executable prototype; public names and behavior are provisional.
Applications depend on package `ex-api` and import its library as `ex`. The
implementation remains in `expansex-core`; the public crate re-exports its types.
This contract describes implemented behavior only. See [architecture](ARCHITECTURE.md)
for planned runtime responsibilities.

## Buffer descriptions

`BufferDescriptor<'a>` borrows a label and a usage slice. `size` is a `u64` byte
count. Multiple roles are permitted; duplicate roles have no extra meaning. An
empty label is valid. `validate()` rejects zero size first, then missing usages.
`BufferDescriptorError` exposes `ZeroSize` and `MissingUsage`, implements
`Display`, and implements `std::error::Error`.

Successful validation means only those two rules passed. It does not establish
backend support, allocation success, data initialization, or synchronization.
The descriptor contains no bytes and owns no GPU storage. Its derived `Copy`
implementation copies numbers and shared references. Actual resource creation
must decide how to retain metadata beyond the borrowed descriptor's lifetime.

## Recording and finishing

`CommandRecorder` owns a `Vec<String>`. `new()` and `default()` start an empty
recording. `insert_debug_marker(&mut self, &str)` stores an owned label copy.
`finish(self)` consumes the recorder and moves the sequence into `CommandBuffer`.
`debug_markers(&self)` returns a borrowed slice in insertion order. Empty command
buffers are allowed; finished buffers are not cloneable in this prototype.

Only labels exist. Finishing never submits, allocates backend command storage, or
executes graphics work. Rust prevents recording through a finished buffer and
using a consumed recorder, but establishes no GPU completion guarantee.

## Verification

Five core unit tests cover descriptor errors, accepted multiple usages, label
ownership/order, and empty recording. A compile-fail documentation test checks
recorder reuse after finishing. A public `ex` documentation example exercises
description validation and recording. The application example runs through `ex`:

```text
cargo run -p ex-api --example api_draft --locked --offline
```

It prints a validated description and stored labels, then explicitly reports
that no GPU memory or execution occurred. A label named 'draw triangle' is text.

## Next contract

Phase 1 must define device creation, allocated buffer ownership, actual recorded
operations, submission order, completion visibility, and resource retention
through completion. Implement and test a small CPU execution path before adding
rasterization. There is no existing device/queue/backend ABI to preserve yet.
