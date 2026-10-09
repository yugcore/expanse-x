# SDK shader import contract

This SDK step lets established
GLSL shader files drive the existing EX software pipeline. It does not introduce
a language, change core execution, or claim general GLSL/SPIR-V compatibility.

## Chosen boundary

Use the installed Khronos glslangValidator to compile GLSL to SPIR-V 1.0, with
Vulkan 1.0 source semantics and optimization disabled. Validate the exact binary
with spirv-val before lowering a deliberately narrow subset into EX IR. Compiler
and validator are build tools; EX still executes every shader instruction itself.
See [glslang](https://github.com/KhronosGroup/glslang),
[SPIRV-Tools](https://github.com/KhronosGroup/SPIRV-Tools), and the
[SPIR-V specification](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html).

The ex-shader Rust library is SDK tooling, separate from the public ex graphics
library. It owns validated instruction arrays, exposes a borrowed ShaderDescriptor,
reports reflection, and emits a Rust module that creates a regular EX Shader.
Expected compiler/import errors use Result with contextual diagnostics.

Support a single main entry point, vertex or fragment, one straight-line function,
32-bit float scalars and vec4 values, finite constants, local values, addition,
multiplication, scalar multiplication, vec4 dot products, component extraction,
vec4 construction and swizzles. Constant component reads are supported; stores
through component pointers are rejected. Inputs and user outputs are vec4 at consecutive
locations starting at zero. Vertex position is mandatory; fragment colour is the
single output at location zero. Interpolation remains EX's existing smooth,
perspective-correct interpolation. Reject alternate interpolation decorations.

Uniforms are descriptor set zero blocks of sequential vec4 members, with offsets
0,16,32,..., using EX's eight binding slots and existing uniform size limit.
Only static member access is supported. Reflection reports names, locations,
stage, uniform slots/minimum byte sizes, and generated instruction count.
Reflect declared interfaces and blocks; do not silently discard incompatible ones.

Reject compute import, matrices, other vector widths, integer arithmetic,
textures, extensions, includes, loops, branches, function calls and unsupported
operations/decorations explicitly. Existing hand-authored EX IR still supports
its broader core operations. SPIR-V validation and supported lowering are distinct
checks: a valid standard shader may be outside this importer.

## Build and developer workflow

Commands: ex shader build FILE --stage vertex|fragment --output FILE.rs;
ex shader import FILE.spv --output FILE.rs; ex shader reflect FILE --stage
vertex|fragment (compile GLSL and print JSON reflection). Output creation is
exclusive and refuses existing files. Compiler/import errors leave no output;
a failed artifact write removes its own partial file, best effort if the OS
denies cleanup.
Tool paths default to PATH and can be selected using EX_GLSLANG and EX_SPIRV_VAL;
invoke executables directly with path arguments, never through a shell.

Generated starters gain shaders/triangle.vert and triangle.frag, plus build.rs
using a local ex-shader build dependency. Cargo compiles shaders into OUT_DIR;
src/shaders.rs includes those generated modules. Track the source files,
shader directory, tool path environment variables, PATH and resolved compiler/
validator executable files for rebuilds. Dependency changes also invalidate
Cargo's build results. Reject includes so external source dependencies cannot
escape the declared build inputs. This uses Cargo's incremental build cache;
no independent persistent artifact cache or portable EX binary format is added.

doctor must also report shader compiler/validator availability. No installation
or machine configuration changes. Generated projects still resolve offline;
installed native shader tools are additionally needed at build time, with no
registry crates. Core-only builds remain independent of those executables.

Source is limited to 4 MiB; SPIR-V to 1 MiB, little-endian version 1.0 and an
ID bound of 1..=65536. Check instruction framing before lowering. Import
the same private snapshot that spirv-val validates. Preserve compiler diagnostics;
unsupported lowering reports SPIR-V opcode and word offset. Temporary directories
are exclusively created; cleanup removes only the attempt's known files.

## Evidence required

Compile/import real shaders; reflect stage locations and uniform byte sizes;
evaluate arithmetic/swizzles/uniforms through EX; reject standard-valid unsupported
features, malformed binaries and bad source. Exercise CLI refusal/diagnostics and
generated starter build/render. Edit GLSL and verify rendered pixels change,
then verify an invalid shader makes the build fail instead of using old output.
Run the workspace checks and update the API catalog.

## Tests and examples (2026-10-10)

The [ex-shader library](../crates/ex-shader/src/lib.rs) implements native tool
invocation, private snapshots, strict lowering, owned reflection and generated
Rust. [CLI dispatch](../crates/ex-sdk/src/shader.rs) implements all three shader
commands. [Starter build.rs](../crates/ex-sdk/templates/build.rs) invokes the
tracked Cargo build helper. No core execution or graphics-library exports changed.

Nine shader-library tests and six SDK CLI tests pass with the installed native
tools. Tests compile/evaluate math and swizzles, reflect and execute uniform blocks,
verify every pixel in two shifted/recoloured frames and a varying gradient,
compare SPIR-V import with compilation, reject malformed/unsupported input and
excessive lowered instruction counts, and exercise file preservation plus build
failure/recovery. The independent starter formats/builds/runs in a path with spaces.
All 272 workspace tests, builds, strict lint checks and Rustdoc checks pass.

Execution uses EX's clip/depth mapping and finite arithmetic rules from
[PIPELINES.md](PIPELINES.md) and [SHADERS.md](SHADERS.md), including runtime overflow
errors. Vulkan compiler source semantics do not promise Vulkan rendering behavior.
The importer trusts the selected installed compiler/validator for their native
contracts, then independently enforces its bounded lowering rules. Native tool
versions are reported by doctor; an exact distributable toolchain pin remains a
release-policy gate.

Verified native versions here: glslang 16.4.0 and SPIRV-Tools v2026.3, with
Rust/Cargo 1.98.1. These are environment evidence, not an exact release pin.
