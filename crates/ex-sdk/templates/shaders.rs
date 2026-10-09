//! GLSL is compiled and validated at build time; execution uses ordinary EX shaders.
use ex::{Device, Shader, ShaderCreateError};

mod vertex_shader {
    include!(concat!(env!("OUT_DIR"), "/vertex.rs"));
}
mod fragment_shader {
    include!(concat!(env!("OUT_DIR"), "/fragment.rs"));
}

pub fn vertex(device: &Device) -> Result<Shader, ShaderCreateError> {
    vertex_shader::create(device)
}
pub fn fragment(device: &Device) -> Result<Shader, ShaderCreateError> {
    fragment_shader::create(device)
}
