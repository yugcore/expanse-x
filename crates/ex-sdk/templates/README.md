# Your EX application

This standalone starter uses the EX source checkout selected during creation.
Its Cargo.toml has an absolute path dependency; update that path if the SDK moves.
Rendering executes on the CPU through EX's software reference target.

```text
cargo run --locked --offline
cargo build --release --locked --offline
cargo fmt -- --check
```

The app writes target/ex-starter.png. Pass another output path after `--`.
Edit shaders/triangle.vert or shaders/triangle.frag to change the GLSL programs,
and src/renderer.rs to change vertices, resources and explicit dependencies.
Install glslangValidator and spirv-val before building (or set EX_GLSLANG and
EX_SPIRV_VAL to their executable paths). build.rs uses the SDK's local ex-shader
build dependency to compile GLSL, validate SPIR-V and generate EX instructions
inside Cargo's OUT_DIR. src/shaders.rs includes those modules. Native tools run
at build time; EX renders the triangle itself. No window system is needed.

The EX-GLSL-1 importer supports straight-line vertex/fragment float/vec4 programs,
consecutive vec4 interfaces and set-zero uniform vec4 blocks. It rejects loops,
branches, functions, includes, textures, other vector sizes and compute import.
See docs/SHADER-IMPORT.md in the selected SDK checkout for exact boundaries.
Existing Float32x2 vertex attributes supply the vec4 position's Z=0 and W=1.
GLSL edits and selected shader-tool changes trigger Cargo rebuilds. Failed shader
compilation stops the build; older shader output is not silently used.

Rendering verifies the corner background and triangle centre before export.
Changing the triangle geometry or matching its colour to the background may
require updating those checks. Result errors describe EX misuse; PNG export
errors describe output I/O after rendering. Read the selected SDK checkout's
api.md, docs/CORE-PROFILE.md and docs/SDK.md for current contracts.

The generated Cargo.lock is for this app; keep it with your project. The stable
toolchain channel follows releases rather than pinning one compiler version.
The release build is a local executable, not a published SDK distribution.
