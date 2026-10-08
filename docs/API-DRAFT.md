# ExpanseX API draft: buffers and recording

Status: exploratory design prototype, not a frozen v0.1 specification.

## Starting point

This prototype covers buffer descriptors and label recording.
No GPU-facing operations are implemented at this stage.

## Current contract

`BufferDescriptor<'a>` borrows a label and a list of usages. `size` is an unsigned 64-bit byte count. Multiple roles may be requested; repeated roles have no additional meaning. An empty label is allowed. `validate()` rejects zero size first, then rejects an empty usage list. Success means those two checks passed; it does not guarantee any device supports the request or that an allocation will succeed.

The descriptor contains no data bytes and owns no GPU memory. Copying it copies its size and borrowed views, not an allocation. The future creation operation must decide how to retain any borrowed metadata it needs after the call.

`CommandRecorder` owns a growable list of string labels. `insert_debug_marker(&mut self, &str)` copies the label into owned storage. `finish(self)` moves that list into a `CommandBuffer` and consumes the recorder. The finished type exposes labels for inspection and no recording methods. It is not cloneable in this draft.

Only labels are recorded. No backend command buffer is allocated. Finishing does not submit or execute anything. Preventing recorder reuse after finishing is one CPU-side invariant; it does not establish safe submission, GPU completion, or resource lifetime handling.

## Reference material

- [Vulkan resource specification](../references/vulkan/Vulkan-Docs/chapters/resources.adoc): buffer creation and resource concepts.
- [Vulkan memory guide](../references/vulkan/Vulkan-Guide/chapters/memory_allocation.adoc): allocation is separate from describing a resource.
- [Vulkan command-buffer specification](../references/vulkan/Vulkan-Docs/chapters/cmdbuffers.adoc): recording and executable states as reference material.

Our two Rust types illustrate a recording-to-finished transition; they do not yet model all Vulkan command-buffer states or native requirements.

## Next design decisions

1. Define device creation and a buffer handle, including how creation can fail.
2. Decide how CPU uploads and backend memory allocation fit the public contract.
3. Define a real recorded operation and how it refers to resources.
4. Define queue submission, completion tracking, and keeping referenced resources alive while GPU work is pending.

Do not add pretend successful GPU operations to the prototype. Document each contract before attaching an actual backend. Complete the broader Phase 1 design before starting GPU-facing implementation.

## Verification

Tests cover descriptor validation, stored-label lifetime and order, and finishing an empty recording. A compile-fail documentation example checks that a finished recorder cannot be reused. The runnable example demonstrates the public types; it makes no rendering claim.
