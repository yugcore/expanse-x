use ex_shader::cargo_build;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cargo_build(
        Path::new("shaders/triangle.vert"),
        ex::ShaderStage::Vertex,
        "vertex.rs",
    )?;
    cargo_build(
        Path::new("shaders/triangle.frag"),
        ex::ShaderStage::Fragment,
        "fragment.rs",
    )?;
    Ok(())
}
