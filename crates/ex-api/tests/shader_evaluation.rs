use ex::{
    Instance, ShaderCreateError, ShaderDescriptor, ShaderExecutionError, ShaderInstruction as Op,
    ShaderStage, ShaderValidationError as Invalid,
};
use std::error::Error;

fn descriptor<'a>(instructions: &'a [Op], outputs: &'a [u32]) -> ShaderDescriptor<'a> {
    ShaderDescriptor {
        label: "test",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions,
        outputs,
    }
}

#[test]
fn operations_outputs_and_stage_interfaces_execute_in_order() {
    let device = Instance::new().default_adapter().create_device();
    let ops = [
        Op::Input { location: 0 },
        Op::Input { location: 1 },
        Op::Add { left: 0, right: 1 },
        Op::Multiply { left: 0, right: 1 },
        Op::Dot4 { left: 0, right: 1 },
        Op::Swizzle {
            value: 2,
            components: [3, 1, 1, 0],
        },
        Op::Constant([-2.0, 0.0, 0.5, 1.0]),
    ];
    let shader = device
        .create_shader(descriptor(&ops, &[5, 3, 4, 6, 0, 5]))
        .unwrap();
    let inputs = [[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]];
    assert_eq!(
        shader.evaluate(&inputs).unwrap(),
        vec![
            [12.0, 8.0, 8.0, 6.0],
            [5.0, 12.0, 21.0, 32.0],
            [70.0; 4],
            [-2.0, 0.0, 0.5, 1.0],
            inputs[0],
            [12.0, 8.0, 8.0, 6.0],
        ]
    );
    let fragment = device
        .create_shader(ShaderDescriptor {
            stage: ShaderStage::Fragment,
            outputs: &[6],
            ..descriptor(&ops, &[6])
        })
        .unwrap();
    // Raw output is neither clamped nor converted to bytes.
    assert_eq!(
        fragment.evaluate(&inputs).unwrap(),
        vec![[-2.0, 0.0, 0.5, 1.0]]
    );
}

#[test]
fn owned_source_identity_and_repeated_concurrent_invocations() {
    let device = Instance::new().default_adapter().create_device();
    let other_device = device.adapter().create_device();
    let shader = {
        let label = String::from("owned shader");
        let mut ops = vec![Op::Input { location: 0 }];
        let mut outputs = vec![0];
        let shader = device
            .create_shader(ShaderDescriptor {
                label: &label,
                input_count: 1,
                ..descriptor(&ops, &outputs)
            })
            .unwrap();
        ops[0] = Op::Constant([99.0; 4]);
        outputs[0] = u32::MAX;
        shader
    };
    assert_eq!(shader.label(), "owned shader");
    assert_eq!(shader.stage(), ShaderStage::Vertex);
    assert_eq!(shader.input_count(), 1);
    assert_eq!(shader.instructions(), &[Op::Input { location: 0 }]);
    assert_eq!(shader.outputs(), &[0]);
    assert!(shader.device().is_same(&device));
    assert!(shader.belongs_to(&device));
    assert!(!shader.belongs_to(&other_device));
    assert!(shader.is_same(&shader.clone()));
    let separate = device
        .create_shader(ShaderDescriptor {
            label: shader.label(),
            stage: shader.stage(),
            input_count: shader.input_count(),
            instructions: shader.instructions(),
            outputs: shader.outputs(),
        })
        .unwrap();
    assert!(!shader.is_same(&separate));
    drop(device);
    std::thread::scope(|scope| {
        for invocation in 0..8 {
            let shader = &shader;
            scope.spawn(move || {
                let input = [invocation as f32; 4];
                for _ in 0..10 {
                    assert_eq!(shader.evaluate(&[input]).unwrap(), vec![input]);
                }
            });
        }
    });
}

#[test]
fn invalid_operand_graphs_are_rejected_before_evaluation() {
    for operand in [0, 1, 2, u32::MAX] {
        for op in [
            Op::Add {
                left: operand,
                right: 0,
            },
            Op::Multiply {
                left: 0,
                right: operand,
            },
            Op::Dot4 {
                left: operand,
                right: 0,
            },
            Op::Swizzle {
                value: operand,
                components: [0, 1, 2, 3],
            },
        ] {
            assert!(matches!(
                descriptor(&[op], &[0]).validate(),
                Err(Invalid::InvalidOperand { instruction: 0, .. })
            ));
        }
    }
    for op in [
        Op::Add { left: 0, right: 1 },
        Op::Multiply { left: 1, right: 0 },
        Op::Dot4 {
            left: 0,
            right: u32::MAX,
        },
    ] {
        assert!(matches!(
            descriptor(&[Op::Constant([0.0; 4]), op], &[1]).validate(),
            Err(Invalid::InvalidOperand { instruction: 1, .. })
        ));
    }
    assert_eq!(
        descriptor(&[], &[0]).validate(),
        Err(Invalid::InvalidOutput {
            output: 0,
            register: 0,
            instruction_count: 0
        })
    );
    assert_eq!(
        descriptor(&[Op::Constant([0.0; 4])], &[0, 9]).validate(),
        Err(Invalid::InvalidOutput {
            output: 1,
            register: 9,
            instruction_count: 1
        })
    );
}

#[test]
fn invalid_input_locations_and_swizzle_lanes_are_descriptive() {
    assert_eq!(
        descriptor(&[Op::Input { location: 2 }], &[0]).validate(),
        Err(Invalid::InvalidInputLocation {
            instruction: 0,
            location: 2,
            input_count: 2
        })
    );
    for lane in 0..4 {
        for component in [4, u8::MAX] {
            let mut components = [0, 1, 2, 3];
            components[lane] = component;
            let ops = [
                Op::Constant([0.0; 4]),
                Op::Swizzle {
                    value: 0,
                    components,
                },
            ];
            assert_eq!(
                descriptor(&ops, &[1]).validate(),
                Err(Invalid::InvalidSwizzle {
                    instruction: 1,
                    lane,
                    component
                })
            );
        }
    }
}

#[test]
fn bounded_programs_accept_maxima_and_reject_excesses() {
    let device = Instance::new().default_adapter().create_device();
    let mut ops = vec![Op::Constant([1.0; 4])];
    for index in 1..256 {
        ops.push(Op::Add {
            left: index - 1,
            right: 0,
        });
    }
    let outputs = [255; 17];
    let maximum = ShaderDescriptor {
        input_count: 16,
        ..descriptor(&ops, &outputs)
    };
    assert_eq!(maximum.validate(), Ok(()));
    assert_eq!(
        device
            .create_shader(maximum)
            .unwrap()
            .evaluate(&[[0.0; 4]; 16])
            .unwrap(),
        vec![[256.0; 4]; 17]
    );
    assert_eq!(
        ShaderDescriptor {
            input_count: 17,
            ..maximum
        }
        .validate(),
        Err(Invalid::TooManyInputs {
            count: 17,
            maximum: 16
        })
    );
    ops.push(Op::Constant([0.0; 4]));
    assert_eq!(
        descriptor(&ops, &[0]).validate(),
        Err(Invalid::TooManyInstructions {
            count: 257,
            maximum: 256
        })
    );
    for stage in [ShaderStage::Vertex, ShaderStage::Fragment] {
        assert_eq!(
            ShaderDescriptor {
                stage,
                ..descriptor(&ops[..1], &[])
            }
            .validate(),
            Err(Invalid::InvalidOutputCount { stage, count: 0 })
        );
    }
    assert_eq!(
        descriptor(&ops[..1], &[0; 18]).validate(),
        Err(Invalid::InvalidOutputCount {
            stage: ShaderStage::Vertex,
            count: 18
        })
    );
    assert_eq!(
        ShaderDescriptor {
            stage: ShaderStage::Fragment,
            ..descriptor(&ops[..1], &[0, 0])
        }
        .validate(),
        Err(Invalid::InvalidOutputCount {
            stage: ShaderStage::Fragment,
            count: 2
        })
    );
}

#[test]
fn non_finite_constants_are_rejected_in_every_lane_and_creation_chains_error() {
    let device = Instance::new().default_adapter().create_device();
    for lane in 0..4 {
        for number in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut value = [0.0; 4];
            value[lane] = number;
            let ops = [Op::Constant(value)];
            assert_eq!(
                descriptor(&ops, &[0]).validate(),
                Err(Invalid::NonFiniteConstant { instruction: 0 })
            );
            let error = device.create_shader(descriptor(&ops, &[0])).unwrap_err();
            assert!(matches!(
                error,
                ShaderCreateError::InvalidDescriptor(Invalid::NonFiniteConstant { instruction: 0 })
            ));
            assert!(error.source().is_some());
            assert!(error.to_string().contains("non-finite constant"));
        }
    }
}

#[test]
fn wrong_input_count_precedes_numeric_checks_and_unused_inputs_are_checked() {
    let device = Instance::new().default_adapter().create_device();
    let shader = device
        .create_shader(descriptor(&[Op::Constant([0.0; 4])], &[0]))
        .unwrap();
    assert!(matches!(
        shader.evaluate(&[[f32::NAN; 4]]),
        Err(ShaderExecutionError::WrongInputCount {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        shader.evaluate(&[[0.0; 4]; 3]),
        Err(ShaderExecutionError::WrongInputCount {
            expected: 2,
            actual: 3
        })
    ));
    for lane in 0..4 {
        let mut input = [0.0; 4];
        input[lane] = f32::INFINITY;
        let error = shader.evaluate(&[[0.0; 4], input]).unwrap_err();
        assert!(matches!(
            error,
            ShaderExecutionError::NonFiniteInput { location: 1 }
        ));
        assert!(error.source().is_none());
    }
    assert_eq!(shader.evaluate(&[[0.0; 4]; 2]).unwrap(), vec![[0.0; 4]]);
}

#[test]
fn overflow_including_unused_results_fails_without_poisoning_program() {
    let device = Instance::new().default_adapter().create_device();
    for op in [
        Op::Add { left: 0, right: 0 },
        Op::Multiply { left: 0, right: 0 },
        Op::Dot4 { left: 0, right: 0 },
    ] {
        let shader = device
            .create_shader(ShaderDescriptor {
                input_count: 1,
                ..descriptor(&[Op::Input { location: 0 }, op], &[0])
            })
            .unwrap();
        let error = shader.evaluate(&[[f32::MAX; 4]]).unwrap_err();
        assert!(matches!(
            error,
            ShaderExecutionError::NonFiniteResult { instruction: 1 }
        ));
        assert!(error.to_string().contains("instruction 1"));
        assert_eq!(shader.evaluate(&[[1.0; 4]]).unwrap(), vec![[1.0; 4]]);
    }
}

#[test]
fn dot_product_has_documented_component_order_and_constant_programs_need_no_inputs() {
    let device = Instance::new().default_adapter().create_device();
    let shader = device
        .create_shader(ShaderDescriptor {
            input_count: 0,
            stage: ShaderStage::Fragment,
            ..descriptor(
                &[
                    Op::Constant([1e20, -1e20, 3.0, 4.0]),
                    Op::Constant([1.0; 4]),
                    Op::Dot4 { left: 0, right: 1 },
                ],
                &[2],
            )
        })
        .unwrap();
    assert_eq!(shader.evaluate(&[]).unwrap(), vec![[7.0; 4]]);
}
