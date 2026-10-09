use ex::*;
use ex_shader::{build_glsl, import_spirv};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    base: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/shader-tests")
            .canonicalize()
            .unwrap_or_else(|_| {
                let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/shader-tests");
                fs::create_dir_all(&base).unwrap();
                base.canonicalize().unwrap()
            });
        let root = base.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root, base }
    }
    fn source(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, format!("#version 450\n{body}\n")).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(root), Ok(base)) = (self.root.canonicalize(), self.base.canonicalize()) {
            if root.parent() == Some(base.as_path()) {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}
fn device() -> Device {
    Instance::new().default_adapter().create_device()
}

#[test]
fn compiled_arithmetic_swizzles_locals_and_varyings_execute_in_ex() {
    let f = Fixture::new();
    let source = f.source("math.vert", "layout(location=0) in vec4 position; layout(location=0) out vec4 tint; void main() { vec4 a = position * 2.0; gl_Position = a + vec4(1.0); tint = a.wzyx; }");
    let module = build_glsl(&source, ShaderStage::Vertex).unwrap();
    assert_eq!(module.reflection().inputs[0].name, "position");
    assert_eq!(module.reflection().outputs[0].location, 0);
    let shader = device().create_shader(module.descriptor()).unwrap();
    drop(module);
    assert_eq!(
        shader.evaluate(&[[1., 2., 3., 4.]]).unwrap(),
        vec![[3., 5., 7., 9.], [8., 6., 4., 2.]]
    );
    let source = f.source("math.frag", "layout(location=0) in vec4 tint; layout(location=0) out vec4 colour; void main() { float a = dot(tint, vec4(1.0,2.0,3.0,4.0)); colour = vec4(a, tint.z, tint.x * 0.5, 1.0); }");
    let module = build_glsl(&source, ShaderStage::Fragment).unwrap();
    let shader = device().create_shader(module.descriptor()).unwrap();
    assert_eq!(
        shader.evaluate(&[[1., 2., 3., 4.]]).unwrap(),
        vec![[30., 3., 0.5, 1.]]
    );
}

#[test]
fn reflected_uniforms_link_and_control_rendered_pixels() {
    let f = Fixture::new();
    let vertex = f.source("uniform.vert", "layout(location=0) in vec4 position; layout(set=0,binding=2,std140) uniform Params { vec4 shift; vec4 colour; } params; void main() { gl_Position=position+params.shift; }");
    let fragment = f.source("uniform.frag", "layout(location=0) out vec4 output_colour; layout(set=0,binding=2,std140) uniform Params { vec4 shift; vec4 colour; } params; void main() { output_colour=vec4(params.colour.x,params.colour.y,params.colour.z,params.colour.w); }");
    let v = build_glsl(&vertex, ShaderStage::Vertex).unwrap();
    let p = build_glsl(&fragment, ShaderStage::Fragment).unwrap();
    let uniform = &p.reflection().uniforms[0];
    assert_eq!(
        (
            uniform.binding,
            uniform.minimum_bytes,
            uniform.name.as_str()
        ),
        (2, 32, "params")
    );
    assert!(p.reflection().to_json().contains("\"minimum_bytes\":32"));
    let d = device();
    let vertex = d.create_shader(v.descriptor()).unwrap();
    let fragment = d.create_shader(p.descriptor()).unwrap();
    assert!(matches!(
        fragment.evaluate(&[]),
        Err(ShaderExecutionError::MissingUniformBinding {
            binding: 2,
            element: 1
        })
    ));
    let layout = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "reflection",
            entries: &[BindGroupLayoutEntry {
                binding: uniform.binding,
                visibility: ShaderVisibility::All,
                ty: BindingType::UniformBuffer {
                    min_size: uniform.minimum_bytes,
                },
            }],
        })
        .unwrap();
    let descriptor = GraphicsPipelineDescriptor {
        label: "imported",
        vertex_shader: &vertex,
        fragment_shader: &fragment,
        vertex_layout: VertexBufferLayout {
            stride: 16,
            attributes: &[VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x4,
            }],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    };
    assert!(d.create_graphics_pipeline(descriptor).is_err());
    let pipeline = d
        .create_graphics_pipeline_with_bindings(
            descriptor,
            GraphicsPipelineOptions::default(),
            None,
            &layout,
        )
        .unwrap();
    let uniform_buffer = d
        .create_buffer(BufferDescriptor {
            label: "params",
            size: 32,
            usages: &[BufferUsage::Uniform],
            host_access: HostAccess::Write,
        })
        .unwrap();
    let group = d
        .create_bind_group(BindGroupDescriptor {
            label: "params",
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 2,
                resource: BindingResource::UniformBuffer(BufferBinding {
                    buffer: uniform_buffer.clone(),
                    offset: 0,
                    size: 32,
                }),
            }],
        })
        .unwrap();
    let vertices = d
        .create_buffer(BufferDescriptor {
            label: "vertices",
            size: 48,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    vertices
        .write(
            0,
            &[
                [-0.75_f32, 0.75, 0., 1.],
                [0.75, 0.75, 0., 1.],
                [0., -0.75, 0., 1.],
            ]
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
        )
        .unwrap();
    let target = d
        .create_framebuffer(FramebufferDescriptor {
            label: "output",
            width: 16,
            height: 16,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    let mut encoder = d.create_command_encoder();
    let mut pass = encoder
        .begin_render_pass(&target, Some([0, 0, 0, 255]))
        .unwrap();
    pass.set_pipeline(&pipeline).unwrap();
    pass.set_bind_group(&group).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    let commands = encoder.finish();
    for (shift, colour) in [(0.0_f32, [0.25, 0.75, 1., 1.]), (0.25, [1., 0.5, 0.25, 1.])] {
        uniform_buffer
            .write(
                0,
                &[[shift, 0., 0., 0.], colour]
                    .into_iter()
                    .flatten()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        d.default_queue().submit(&commands).unwrap();
        let mut pixels = vec![[0; 4]; 256];
        target.read_pixels(&mut pixels).unwrap();
        let expected = colour.map(|v| (v * 255.0).round() as u8);
        assert_eq!(pixels[8 * 16 + 8], expected);
        assert_eq!(pixels.iter().filter(|p| **p == expected).count(), 72);
        // Independent screen-space geometry oracle: changing vertex uniforms
        // translates the triangle, while the fragment uniform changes its colour.
        for y in 0..16 {
            for x in 0..16 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let left = 2.0 + shift * 8.0 + (py - 2.0) * 0.5;
                let right = 14.0 + shift * 8.0 - (py - 2.0) * 0.5;
                let covered = (2.0..14.0).contains(&py) && px >= left && px < right;
                assert_eq!(
                    pixels[y * 16 + x],
                    if covered { expected } else { [0, 0, 0, 255] }
                );
            }
        }
    }
}

#[test]
fn binary_import_matches_compiler_route_and_owns_its_source() {
    let f = Fixture::new();
    let source = f.source(
        "colour.frag",
        "layout(location=0) out vec4 colour; void main() { colour=vec4(0.25,0.5,0.75,1.0); }",
    );
    let binary = f.root.join("colour.spv");
    let out =
        Command::new(std::env::var_os("EX_GLSLANG").unwrap_or_else(|| "glslangValidator".into()))
            .args(["-V", "--target-env", "vulkan1.0", "-Od", "-S", "frag", "-o"])
            .arg(&binary)
            .arg(&source)
            .output()
            .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let imported = import_spirv(&binary).unwrap();
    let compiled = build_glsl(&source, ShaderStage::Fragment).unwrap();
    assert_eq!(imported.reflection(), compiled.reflection());
    assert_eq!(
        imported.descriptor().instructions,
        compiled.descriptor().instructions
    );
    fs::write(&binary, b"changed").unwrap();
    assert_eq!(
        device()
            .create_shader(imported.descriptor())
            .unwrap()
            .evaluate(&[])
            .unwrap(),
        vec![[0.25, 0.5, 0.75, 1.]]
    );
}

#[test]
fn valid_standard_shaders_outside_subset_fail_explicitly() {
    let f = Fixture::new();
    for (name, body) in [
        ("loop.frag", "layout(location=0) out vec4 colour; void main() { colour=vec4(0.0); for(int i=0;i<2;i++) { colour+=vec4(0.25); } }"),
        ("vec3.frag", "layout(location=0) out vec4 colour; void main() { vec3 c=vec3(1.0); colour=vec4(c,1.0); }"),
        ("flat.frag", "layout(location=0) flat in vec4 tint; layout(location=0) out vec4 colour; void main() { colour=tint; }"),
        ("set.frag", "layout(set=1,binding=0,std140) uniform Params { vec4 c; } params; layout(location=0) out vec4 colour; void main() { colour=params.c; }"),
        ("texture.frag", "layout(set=0,binding=0) uniform sampler2D image; layout(location=0) out vec4 colour; void main() { colour=texture(image,vec2(0.5)); }"),
        ("function.frag", "layout(location=0) out vec4 colour; vec4 c() { return vec4(1.0); } void main() { colour=c(); }"),
        ("include.frag", "#extension GL_GOOGLE_include_directive : enable\nlayout(location=0) out vec4 colour; void main() { colour=vec4(1.0); }"),
        ("location.frag", "layout(location=1) out vec4 colour; void main() { colour=vec4(1.0); }"),
    ] {
        let source = f.source(name,body);
        let error = build_glsl(&source,ShaderStage::Fragment).unwrap_err().to_string();
        assert!(error.contains("EX-GLSL-1") && error.contains("SPIR-V word"), "{name}: {error}");
        assert!(!error.contains("glslangValidator failed"), "shader should be valid standard GLSL: {error}");
    }
}

#[test]
fn malformed_binary_source_and_compute_return_errors() {
    let f = Fixture::new();
    let binary = f.root.join("bad.spv");
    for bytes in [
        vec![],
        b"garbage".to_vec(),
        [0x0723_0203u32, 0x0001_0000, 0, 10, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect(),
    ] {
        fs::write(&binary, bytes).unwrap();
        assert!(import_spirv(&binary).is_err());
    }
    let source = f.source("bad.frag", "void main() { broken } ");
    let error = build_glsl(&source, ShaderStage::Fragment)
        .unwrap_err()
        .to_string();
    assert!(error.contains("ERROR") && error.contains("bad.frag"));
    assert!(build_glsl(&source, ShaderStage::Compute)
        .unwrap_err()
        .to_string()
        .contains("compute shader import"));
}

#[test]
fn lowered_instruction_budget_is_enforced() {
    let f = Fixture::new();
    let body = format!("layout(location=0) in vec4 tint; layout(location=0) out vec4 colour; void main() {{ vec4 c=tint; {} colour=c; }}", "c=c*tint;".repeat(150));
    let source = f.source("large.frag", &body);
    assert!(build_glsl(&source, ShaderStage::Fragment)
        .unwrap_err()
        .to_string()
        .contains("256 instruction limit"));
}
