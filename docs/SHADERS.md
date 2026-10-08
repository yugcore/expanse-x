# EX shader representation and interpreter

EX supplies owned, validated vertex and fragment programs and a CPU interpreter
callable through `Shader::evaluate`. Programs also execute through recorded
graphics pipelines, vertex layouts, and perspective-correct interpolation; see
[PIPELINES.md](PIPELINES.md). Fragment texture sampling is also implemented under
[TEXTURES.md](TEXTURES.md). The fixed-function path remains available.

Vulkan accepts SPIR-V shader modules and defines interfaces between shader stages
([shader contract](https://docs.vulkan.org/spec/latest/chapters/shaders.html),
[interfaces](https://docs.vulkan.org/spec/latest/chapters/interfaces.html)). EX
starts with its own small instruction representation so validation, execution,
and later diagnostics have the same inspectable source. This is a foundation,
not a new shader language or a full compiler.

## Execution rules

- Every value is `[f32; 4]`. An instruction's position is its result register:
  instruction 0 creates register 0, instruction 1 creates register 1, and so on.
- Instructions are Input, Constant, componentwise Add and Multiply, Dot4
  (four-component dot product broadcast into all lanes), Swizzle, and fragment
  SampleTexture { binding, coordinates } (XY of a prior register).
- Operands must reference earlier results. There are no writes, branches, loops,
  buffer accesses, uniforms, built-in inputs, or arbitrary Rust callbacks. Texture
  sampling supports eight paired slots, normalized view-local coordinates, and
  nearest/bilinear clamp/repeat; no derivatives or LOD exists.
- A descriptor borrows a label, instructions, and output register indices.
  `Device::create_shader` validates and copies them into immutable shared state.
  Clones retain the same identity and device; equal programs created separately
  have separate identities. Evaluation needs no device gate or mutable storage.
- At most 16 input vectors, 256 instructions, and 17 vertex outputs are accepted.
  A vertex program requires output 0 (reserved for homogeneous clip position
  x,y,z,w) and may return up to 16 additional vectors (varyings). A fragment
  program returns exactly one RGBA vector. Output indices can repeat.
- These stage interfaces are linked when creating a graphics pipeline. Standalone
  evaluation returns raw vectors: it performs no projection, clipping, interpolation, colour
  clamping, byte conversion, or stage compatibility check.
- All constants, all supplied inputs (even unused ones), and every instruction
  result must be finite. Arithmetic uses f32 in instruction order; Dot4 sums
  products from x through w with ordinary multiplication/addition, no explicit
  fused operation. Overflow is an execution error, with its instruction index.
  No hardware bitwise agreement or shader performance claim is made.

## Validation and execution

Descriptor validation checks input count, instruction count, stage output count,
then each instruction in order (input location, finite constant, prior operand,
swizzle lane, sampling stage/slot/register), then output indices. No allocation
is needed for validation. SampleTexture requires Fragment stage and binding < 8.
Referenced indices are u32; swizzle lanes are u8 in 0..4. Self references, forward
references, cycles, missing outputs, and out-of-range inputs are rejected.

Creation errors distinguish invalid descriptors from failed reservation for
owned instructions, outputs, or label. Small Arc allocation follows Rust's
ordinary allocation behavior; comprehensive OOM recovery is not promised.

Evaluation checks exact input count, checks finite inputs in order, interprets
into a bounded local register array, rejects the first non-finite result, then
reserves and returns outputs in descriptor order. Errors return no partial
output. Allocation failure includes the reservation source. Immutable programs
can be evaluated repeatedly or concurrently without changing resources.
Standalone evaluation encounters MissingTextureBinding at a sample instruction;
bound rendering supplies view snapshots, while TextureView::sample offers
synchronized standalone sampling. All sample instructions contribute required
slots even when their results are unused. Existing descriptor literals need no
new field, and arithmetic-only evaluation remains compatible.

```rust
use ex::{Instance, ShaderDescriptor, ShaderInstruction as Op, ShaderStage};
let device = Instance::new().default_adapter().create_device();
let shader = device.create_shader(ShaderDescriptor {
    label: "translate position", stage: ShaderStage::Vertex, input_count: 1,
    instructions: &[
        Op::Input { location: 0 },                    // r0 = supplied position
        Op::Constant([0.25, 0.0, 0.0, 0.0]),           // r1 = translation
        Op::Add { left: 0, right: 1 },                // r2 = r0 + r1
    ],
    outputs: &[2],
})?;
assert_eq!(shader.evaluate(&[[0.0, 0.5, 0.0, 1.0]])?,
           vec![[0.25, 0.5, 0.0, 1.0]]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The executable [shader example](../crates/ex-api/examples/shader_evaluation.rs)
also demonstrates a fragment colour transformation. Public tests check operations,
invalid graphs/interfaces, bounded execution, numeric errors, owned metadata,
device identity, and concurrent evaluation. The internal lifetime test checks
cleanup after the last clone. Allocation failures are handled without forcing
the process to exhaust memory.

The [programmable triangle](../crates/ex-api/examples/programmable_triangle.rs)
connects these operations to rendered pixels. Internal renderer invocations fill
fixed output arrays without heap allocation per vertex/fragment. Standalone
evaluation still returns an owned Vec and does not itself render.

## Source-language direction: accepted

Use established shader languages/toolchains when external shader ingestion is
added; reconsider a custom EX source language only for a demonstrated benefit.
A possible route is HLSL/GLSL -> SPIR-V -> validated EX IR -> EX execution.
[DXC](https://github.com/microsoft/DirectXShaderCompiler) supports HLSL-to-SPIR-V;
[glslang](https://github.com/KhronosGroup/glslang) supports GLSL-to-SPIR-V. The
importer and supported subset are future work, not current language support.
EX's current vector instructions need to grow before general shader import is
possible. This decision preserves EX's ownership of rendering and execution.
