use ex::{Instance, ShaderDescriptor, ShaderInstruction as Op, ShaderStage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Instance::new().default_adapter().create_device();
    let vertex = device.create_shader(ShaderDescriptor {
        label: "translate position",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[
            Op::Input { location: 0 },
            Op::Constant([0.25, 0.0, 0.0, 0.0]),
            Op::Add { left: 0, right: 1 },
            Op::Input { location: 1 },
        ],
        outputs: &[2, 3], // clip position, then a colour varying
    })?;
    let output = vertex.evaluate(&[[0.0, 0.5, 0.0, 1.0], [0.8, 0.4, 0.2, 1.0]])?;
    assert_eq!(output, vec![[0.25, 0.5, 0.0, 1.0], [0.8, 0.4, 0.2, 1.0]]);

    let fragment = device.create_shader(ShaderDescriptor {
        label: "tint colour",
        stage: ShaderStage::Fragment,
        input_count: 1,
        instructions: &[
            Op::Input { location: 0 },
            Op::Constant([0.5, 1.0, 0.5, 1.0]),
            Op::Multiply { left: 0, right: 1 },
        ],
        outputs: &[2],
    })?;
    let colour = fragment.evaluate(&[output[1]])?;
    assert_eq!(colour, vec![[0.4, 0.4, 0.1, 1.0]]);
    println!(
        "EX vertex position: {:?}; fragment RGBA: {:?}",
        output[0], colour[0]
    );
    println!("Independent shader evaluation; pipeline drawing is the next step.");
    Ok(())
}
