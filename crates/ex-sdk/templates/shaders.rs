//! Editable current EX IR. GLSL/HLSL/SPIR-V importing is future SDK work.
use ex::{
    Device, Shader, ShaderCreateError, ShaderDescriptor, ShaderInstruction as Op, ShaderStage,
};

pub fn vertex(device: &Device) -> Result<Shader, ShaderCreateError> {
    device.create_shader(ShaderDescriptor {
        label: "starter positions",
        stage: ShaderStage::Vertex,
        input_count: 1,
        instructions: &[Op::Input { location: 0 }],
        outputs: &[0],
    })
}
pub fn fragment(device: &Device) -> Result<Shader, ShaderCreateError> {
    device.create_shader(ShaderDescriptor {
        label: "starter colour",
        stage: ShaderStage::Fragment,
        input_count: 0,
        instructions: &[Op::Constant([0.25, 0.75, 1.0, 1.0])],
        outputs: &[0],
    })
}
