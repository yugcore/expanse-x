use ex::*;
use ex_shader::build_glsl;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
#[test]
fn compiled_varyings_interpolate_into_every_rendered_sample() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/shader-varying-tests");
    fs::create_dir_all(&base).unwrap();
    let root = base.join(format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    // Non-recursive cleanup removes only the two source files created here.
    struct Sources(std::path::PathBuf);
    impl Drop for Sources {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0.join("sample.vert"));
            let _ = fs::remove_file(self.0.join("sample.frag"));
            let _ = fs::remove_dir(&self.0);
        }
    }
    let sources = Sources(root);
    let vertex = sources.0.join("sample.vert");
    let fragment = sources.0.join("sample.frag");
    fs::write(&vertex,"#version 450\nlayout(location=0) in vec4 position; layout(location=0) out vec4 tint; void main() { gl_Position=position; tint=position*0.5+vec4(0.5); }\n").unwrap();
    fs::write(&fragment,"#version 450\nlayout(location=0) in vec4 tint; layout(location=0) out vec4 colour; void main() { colour=tint; }\n").unwrap();
    let v = build_glsl(&vertex, ShaderStage::Vertex).unwrap();
    let f = build_glsl(&fragment, ShaderStage::Fragment).unwrap();
    let d = Instance::new().default_adapter().create_device();
    let v = d.create_shader(v.descriptor()).unwrap();
    let f = d.create_shader(f.descriptor()).unwrap();
    let pipeline = d
        .create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "GLSL varyings",
            vertex_shader: &v,
            fragment_shader: &f,
            vertex_layout: VertexBufferLayout {
                stride: 8,
                attributes: &[VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x2,
                }],
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    let vertices = d
        .create_buffer(BufferDescriptor {
            label: "xy positions",
            size: 24,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    vertices
        .write(
            0,
            &[[-0.75_f32, 0.75], [0.75, 0.75], [0., -0.75]]
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let target = d
        .create_framebuffer(FramebufferDescriptor {
            label: "gradient",
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
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&encoder.finish()).unwrap();
    let mut pixels = vec![[0; 4]; 256];
    target.read_pixels(&mut pixels).unwrap();
    let mut coverage = 0;
    for y in 0..16 {
        for x in 0..16 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let covered = (2.0..14.0).contains(&py)
                && px >= 2.0 + (py - 2.0) * 0.5
                && px < 14.0 - (py - 2.0) * 0.5;
            let expected = if covered {
                coverage += 1;
                [
                    (px * 255. / 16.).round() as u8,
                    ((1. - py / 16.) * 255.).round() as u8,
                    128,
                    255,
                ]
            } else {
                [0, 0, 0, 255]
            };
            assert_eq!(pixels[y * 16 + x], expected, "pixel {x},{y}");
        }
    }
    assert_eq!(coverage, 72);
}
