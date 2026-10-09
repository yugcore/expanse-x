use crate::Device;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

const MAX_INPUTS: u32 = 16;
const MAX_INSTRUCTIONS: usize = 256;
const MAX_VERTEX_OUTPUTS: usize = 17;

/// Stage interface meaning; evaluation itself returns raw vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    /// Output zero is homogeneous clip position; remaining outputs are varyings.
    Vertex,
    /// Exactly one output vector, containing RGBA colour.
    Fragment,
    /// Four built-in input vectors; 1..=8 storage output vectors.
    Compute,
}

/// An EX instruction producing one four-component f32 result.
///
/// Its index in the instruction list is its register number. Operands must
/// reference earlier instructions. Every constant and evaluated value is finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderInstruction {
    /// Read one supplied input vector by location.
    Input { location: u32 },
    /// Read a static float4 element from a uniform buffer in any stage.
    LoadUniform { binding: u32, element: u32 },
    /// Read this compute invocation's float4 element from a storage slot.
    LoadStorage { binding: u32 },
    /// Sample a fragment texture/sampler slot using XY of an earlier register.
    SampleTexture { binding: u32, coordinates: u32 },
    /// Produce a finite literal vector.
    Constant([f32; 4]),
    /// Add corresponding components.
    Add { left: u32, right: u32 },
    /// Multiply corresponding components.
    Multiply { left: u32, right: u32 },
    /// Sum x/y/z/w products in order and broadcast the scalar into all lanes.
    Dot4 { left: u32, right: u32 },
    /// Select source components in order; lanes must each be in 0..4.
    Swizzle { value: u32, components: [u8; 4] },
}

/// Borrowed shader source, copied into immutable owned state on creation.
#[derive(Debug, Clone, Copy)]
pub struct ShaderDescriptor<'a> {
    /// Borrowed debug label; creation owns a copy and empty labels are allowed.
    pub label: &'a str,
    /// Stage interface meaning, checked when validating output count.
    pub stage: ShaderStage,
    /// Exact input count, at most 16; compute requires four built-in vectors.
    pub input_count: u32,
    /// At most 256 instructions; each result is addressed by its list index.
    pub instructions: &'a [ShaderInstruction],
    /// Output registers: 1..=17 vertex, exactly 1 fragment, 1..=8 compute.
    pub outputs: &'a [u32],
}

/// Structural or constant-value error, reported before a shader can be created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderValidationError {
    InvalidUniformLoad {
        instruction: usize,
        binding: u32,
        element: u32,
    },
    StorageLoadingStage {
        instruction: usize,
        stage: ShaderStage,
    },
    InvalidStorageBinding {
        instruction: usize,
        binding: u32,
    },
    ComputeInputCount {
        count: u32,
    },
    TextureSamplingStage {
        instruction: usize,
        stage: ShaderStage,
    },
    InvalidTextureBinding {
        instruction: usize,
        binding: u32,
    },
    TooManyInputs {
        count: u32,
        maximum: u32,
    },
    TooManyInstructions {
        count: usize,
        maximum: usize,
    },
    InvalidOutputCount {
        stage: ShaderStage,
        count: usize,
    },
    InvalidInputLocation {
        instruction: usize,
        location: u32,
        input_count: u32,
    },
    NonFiniteConstant {
        instruction: usize,
    },
    InvalidOperand {
        instruction: usize,
        operand: u32,
    },
    InvalidSwizzle {
        instruction: usize,
        lane: usize,
        component: u8,
    },
    InvalidOutput {
        output: usize,
        register: u32,
        instruction_count: usize,
    },
}

impl fmt::Display for ShaderValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUniformLoad { instruction, binding, element } => write!(f, "instruction {instruction} uniform slot {binding}/element {element} requires slot 0..8 and element 0..256"),
            Self::StorageLoadingStage { instruction, stage } => write!(f, "instruction {instruction} loads storage in {stage:?}; only Compute supports storage"),
            Self::InvalidStorageBinding { instruction, binding } => write!(f, "instruction {instruction} uses storage slot {binding}; valid slots are 0..8"),
            Self::ComputeInputCount { count } => write!(f, "compute shader needs four built-in input vectors, received {count}"),
            Self::TextureSamplingStage { instruction, stage } => write!(f, "instruction {instruction} samples a texture in {stage:?}; only Fragment supports sampling"),
            Self::InvalidTextureBinding { instruction, binding } => write!(f, "instruction {instruction} uses texture slot {binding}; valid slots are 0..8"),
            Self::TooManyInputs { count, maximum } => write!(f, "shader requests {count} inputs; maximum is {maximum}"),
            Self::TooManyInstructions { count, maximum } => write!(f, "shader has {count} instructions; maximum is {maximum}"),
            Self::InvalidOutputCount { stage, count } => write!(f, "{stage:?} shader has invalid output count {count}; vertex requires 1..=17, fragment requires 1, compute requires 1..=8"),
            Self::InvalidInputLocation { instruction, location, input_count } => write!(f, "instruction {instruction} reads input {location}, outside {input_count} inputs"),
            Self::NonFiniteConstant { instruction } => write!(f, "instruction {instruction} contains a non-finite constant"),
            Self::InvalidOperand { instruction, operand } => write!(f, "instruction {instruction} references register {operand}; operands must reference earlier instructions"),
            Self::InvalidSwizzle { instruction, lane, component } => write!(f, "instruction {instruction} swizzle lane {lane} selects component {component}, outside 0..4"),
            Self::InvalidOutput { output, register, instruction_count } => write!(f, "output {output} references register {register}, outside {instruction_count} instructions"),
        }
    }
}
impl Error for ShaderValidationError {}

impl ShaderDescriptor<'_> {
    /// Check bounded interfaces, finite literals, and an acyclic register graph.
    /// No allocation or execution takes place.
    pub fn validate(&self) -> Result<(), ShaderValidationError> {
        if self.input_count > MAX_INPUTS {
            return Err(ShaderValidationError::TooManyInputs {
                count: self.input_count,
                maximum: MAX_INPUTS,
            });
        }
        if self.instructions.len() > MAX_INSTRUCTIONS {
            return Err(ShaderValidationError::TooManyInstructions {
                count: self.instructions.len(),
                maximum: MAX_INSTRUCTIONS,
            });
        }
        if self.stage == ShaderStage::Compute && self.input_count != 4 {
            return Err(ShaderValidationError::ComputeInputCount {
                count: self.input_count,
            });
        }
        let valid_outputs = match self.stage {
            ShaderStage::Vertex => (1..=MAX_VERTEX_OUTPUTS).contains(&self.outputs.len()),
            ShaderStage::Fragment => self.outputs.len() == 1,
            ShaderStage::Compute => (1..=8).contains(&self.outputs.len()),
        };
        if !valid_outputs {
            return Err(ShaderValidationError::InvalidOutputCount {
                stage: self.stage,
                count: self.outputs.len(),
            });
        }
        for (instruction, op) in self.instructions.iter().enumerate() {
            let check_operand = |operand: u32| {
                if operand as usize >= instruction {
                    Err(ShaderValidationError::InvalidOperand {
                        instruction,
                        operand,
                    })
                } else {
                    Ok(())
                }
            };
            match *op {
                ShaderInstruction::LoadUniform { binding, element } => {
                    if binding >= 8 || element >= 256 {
                        return Err(ShaderValidationError::InvalidUniformLoad {
                            instruction,
                            binding,
                            element,
                        });
                    }
                }
                ShaderInstruction::LoadStorage { binding } => {
                    if self.stage != ShaderStage::Compute {
                        return Err(ShaderValidationError::StorageLoadingStage {
                            instruction,
                            stage: self.stage,
                        });
                    }
                    if binding >= 8 {
                        return Err(ShaderValidationError::InvalidStorageBinding {
                            instruction,
                            binding,
                        });
                    }
                }
                ShaderInstruction::SampleTexture {
                    binding,
                    coordinates,
                } => {
                    if self.stage != ShaderStage::Fragment {
                        return Err(ShaderValidationError::TextureSamplingStage {
                            instruction,
                            stage: self.stage,
                        });
                    }
                    if binding >= 8 {
                        return Err(ShaderValidationError::InvalidTextureBinding {
                            instruction,
                            binding,
                        });
                    }
                    check_operand(coordinates)?;
                }
                ShaderInstruction::Input { location } if location >= self.input_count => {
                    return Err(ShaderValidationError::InvalidInputLocation {
                        instruction,
                        location,
                        input_count: self.input_count,
                    });
                }
                ShaderInstruction::Constant(value) if !finite(&value) => {
                    return Err(ShaderValidationError::NonFiniteConstant { instruction });
                }
                ShaderInstruction::Add { left, right }
                | ShaderInstruction::Multiply { left, right }
                | ShaderInstruction::Dot4 { left, right } => {
                    check_operand(left)?;
                    check_operand(right)?;
                }
                ShaderInstruction::Swizzle { value, components } => {
                    check_operand(value)?;
                    for (lane, component) in components.into_iter().enumerate() {
                        if component >= 4 {
                            return Err(ShaderValidationError::InvalidSwizzle {
                                instruction,
                                lane,
                                component,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        for (output, &register) in self.outputs.iter().enumerate() {
            if register as usize >= self.instructions.len() {
                return Err(ShaderValidationError::InvalidOutput {
                    output,
                    register,
                    instruction_count: self.instructions.len(),
                });
            }
        }
        Ok(())
    }
}

/// Shader creation failed before returning an owned program.
#[derive(Debug)]
pub enum ShaderCreateError {
    InvalidDescriptor(ShaderValidationError),
    AllocationFailed { source: TryReserveError },
}
impl fmt::Display for ShaderCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create shader: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve owned shader source: {source}")
            }
        }
    }
}
impl Error for ShaderCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(source) => Some(source),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}

/// Evaluation failed; no partial output is returned and no resource is modified.
#[derive(Debug)]
pub enum ShaderExecutionError {
    MissingUniformBinding { binding: u32, element: u32 },
    MissingStorageBinding { binding: u32 },
    MissingTextureBinding { binding: u32 },
    WrongInputCount { expected: u32, actual: usize },
    NonFiniteInput { location: usize },
    NonFiniteResult { instruction: usize },
    AllocationFailed { source: TryReserveError },
}
impl fmt::Display for ShaderExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUniformBinding { binding, element } => write!(f, "uniform slot {binding} element {element} is unavailable; use layout-aware bound execution"),
            Self::MissingStorageBinding { binding } => write!(
                f,
                "shader storage load needs slot {binding}; use compute dispatch"
            ),
            Self::MissingTextureBinding { binding } => write!(
                f,
                "shader sampling needs texture/sampler slot {binding}; use bound rendering"
            ),
            Self::WrongInputCount { expected, actual } => write!(
                f,
                "shader requires {expected} input vectors, received {actual}"
            ),
            Self::NonFiniteInput { location } => {
                write!(f, "shader input {location} contains a non-finite component")
            }
            Self::NonFiniteResult { instruction } => write!(
                f,
                "shader instruction {instruction} produced a non-finite result"
            ),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve shader evaluation outputs: {source}")
            }
        }
    }
}
impl Error for ShaderExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ShaderState {
    device: Device,
    label: String,
    stage: ShaderStage,
    input_count: u32,
    instructions: Vec<ShaderInstruction>,
    outputs: Vec<u32>,
}

/// An immutable, validated EX program retaining its creating device.
/// Clones share identity and source. Evaluation uses private local registers.
#[derive(Debug, Clone)]
pub struct Shader {
    state: Arc<ShaderState>,
}

impl Device {
    /// Validate and copy shader source; this does not bind a pipeline or render.
    pub fn create_shader(
        &self,
        descriptor: ShaderDescriptor<'_>,
    ) -> Result<Shader, ShaderCreateError> {
        descriptor
            .validate()
            .map_err(ShaderCreateError::InvalidDescriptor)?;
        let allocation = |source| ShaderCreateError::AllocationFailed { source };
        let mut instructions = Vec::new();
        instructions
            .try_reserve_exact(descriptor.instructions.len())
            .map_err(allocation)?;
        instructions.extend_from_slice(descriptor.instructions);
        let mut outputs = Vec::new();
        outputs
            .try_reserve_exact(descriptor.outputs.len())
            .map_err(allocation)?;
        outputs.extend_from_slice(descriptor.outputs);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(allocation)?;
        label.push_str(descriptor.label);
        Ok(Shader {
            state: Arc::new(ShaderState {
                device: self.clone(),
                label,
                stage: descriptor.stage,
                input_count: descriptor.input_count,
                instructions,
                outputs,
            }),
        })
    }
}

impl Shader {
    /// Borrow the owned debug label.
    pub fn label(&self) -> &str {
        &self.state.label
    }
    /// Get stage interface meaning.
    pub fn stage(&self) -> ShaderStage {
        self.state.stage
    }
    /// Get the exact input vector count.
    pub fn input_count(&self) -> u32 {
        self.state.input_count
    }
    /// Borrow the immutable instructions in execution order.
    pub fn instructions(&self) -> &[ShaderInstruction] {
        &self.state.instructions
    }
    /// Borrow output register indices in return order.
    pub fn outputs(&self) -> &[u32] {
        &self.state.outputs
    }
    /// Retain the original creating device.
    pub fn device(&self) -> Device {
        self.state.device.clone()
    }
    /// Check logical device membership.
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.state.device.is_same(device)
    }
    /// Check shared program identity, rather than source equality.
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Interpret one invocation, returning raw output vectors in declared order.
    /// All inputs and results must be finite. No projection or colour conversion
    /// occurs, and fixed-function rendering is unaffected by this call.
    pub fn evaluate(&self, inputs: &[[f32; 4]]) -> Result<Vec<[f32; 4]>, ShaderExecutionError> {
        let mut values = [[0.0; 4]; MAX_VERTEX_OUTPUTS];
        self.evaluate_into(inputs, &mut values)?;
        let mut outputs = Vec::new();
        outputs
            .try_reserve_exact(self.outputs().len())
            .map_err(|source| ShaderExecutionError::AllocationFailed { source })?;
        outputs.extend_from_slice(&values[..self.outputs().len()]);
        Ok(outputs)
    }

    /// Internal invocations avoid allocation per vertex/sample. Callers supply
    /// space for all declared outputs; immutable validated pipelines prove sizes.
    pub(crate) fn evaluate_into(
        &self,
        inputs: &[[f32; 4]],
        outputs: &mut [[f32; 4]],
    ) -> Result<(), ShaderExecutionError> {
        self.evaluate_with_sampler(inputs, outputs, |binding, _| {
            Err(ShaderExecutionError::MissingTextureBinding { binding })
        })
    }

    pub(crate) fn required_uniforms(&self) -> u8 {
        self.instructions().iter().fold(0, |mask, op| match *op {
            ShaderInstruction::LoadUniform { binding, .. } => mask | (1 << binding),
            _ => mask,
        })
    }

    pub(crate) fn required_textures(&self) -> u8 {
        self.instructions().iter().fold(0, |mask, op| match *op {
            ShaderInstruction::SampleTexture { binding, .. } => mask | (1 << binding),
            _ => mask,
        })
    }

    pub(crate) fn evaluate_with_sampler(
        &self,
        inputs: &[[f32; 4]],
        outputs: &mut [[f32; 4]],
        sample: impl Fn(u32, [f32; 2]) -> Result<[f32; 4], ShaderExecutionError>,
    ) -> Result<(), ShaderExecutionError> {
        self.evaluate_with_resources(inputs, outputs, sample, |binding| {
            Err(ShaderExecutionError::MissingStorageBinding { binding })
        })
    }

    pub(crate) fn evaluate_with_resources(
        &self,
        inputs: &[[f32; 4]],
        outputs: &mut [[f32; 4]],
        sample: impl Fn(u32, [f32; 2]) -> Result<[f32; 4], ShaderExecutionError>,
        load: impl Fn(u32) -> Result<[f32; 4], ShaderExecutionError>,
    ) -> Result<(), ShaderExecutionError> {
        self.evaluate_bound(inputs, outputs, sample, load, |binding, element| {
            Err(ShaderExecutionError::MissingUniformBinding { binding, element })
        })
    }
    pub(crate) fn evaluate_bound(
        &self,
        inputs: &[[f32; 4]],
        outputs: &mut [[f32; 4]],
        sample: impl Fn(u32, [f32; 2]) -> Result<[f32; 4], ShaderExecutionError>,
        load: impl Fn(u32) -> Result<[f32; 4], ShaderExecutionError>,
        uniform: impl Fn(u32, u32) -> Result<[f32; 4], ShaderExecutionError>,
    ) -> Result<(), ShaderExecutionError> {
        if inputs.len() != self.input_count() as usize {
            return Err(ShaderExecutionError::WrongInputCount {
                expected: self.input_count(),
                actual: inputs.len(),
            });
        }
        for (location, input) in inputs.iter().enumerate() {
            if !finite(input) {
                return Err(ShaderExecutionError::NonFiniteInput { location });
            }
        }
        // Creation proved all operand/output bounds and a maximum of 256 results.
        // State is immutable, so those invariants hold for every invocation.
        let mut registers = [[0.0_f32; 4]; MAX_INSTRUCTIONS];
        for (instruction, op) in self.instructions().iter().enumerate() {
            let value = match *op {
                ShaderInstruction::LoadUniform { binding, element } => uniform(binding, element)?,
                ShaderInstruction::Input { location } => inputs[location as usize],
                ShaderInstruction::LoadStorage { binding } => load(binding)?,
                ShaderInstruction::SampleTexture {
                    binding,
                    coordinates,
                } => {
                    let uv = registers[coordinates as usize];
                    sample(binding, [uv[0], uv[1]])?
                }
                ShaderInstruction::Constant(value) => value,
                ShaderInstruction::Add { left, right } => std::array::from_fn(|lane| {
                    registers[left as usize][lane] + registers[right as usize][lane]
                }),
                ShaderInstruction::Multiply { left, right } => std::array::from_fn(|lane| {
                    registers[left as usize][lane] * registers[right as usize][lane]
                }),
                ShaderInstruction::Dot4 { left, right } => {
                    let left = registers[left as usize];
                    let right = registers[right as usize];
                    let mut sum = left[0] * right[0];
                    for lane in 1..4 {
                        sum += left[lane] * right[lane];
                    }
                    [sum; 4]
                }
                ShaderInstruction::Swizzle { value, components } => {
                    components.map(|lane| registers[value as usize][lane as usize])
                }
            };
            if !finite(&value) {
                return Err(ShaderExecutionError::NonFiniteResult { instruction });
            }
            registers[instruction] = value;
        }
        for (output, &register) in outputs.iter_mut().zip(self.outputs()) {
            *output = registers[register as usize];
        }
        Ok(())
    }
}

fn finite(value: &[f32; 4]) -> bool {
    value.iter().all(|component| component.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Instance;

    #[test]
    fn last_clone_releases_source_and_retained_device() {
        let device = Instance::new().default_adapter().create_device();
        let shader = device
            .create_shader(ShaderDescriptor {
                label: "retained",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let state = Arc::downgrade(&shader.state);
        let retained = shader.clone();
        drop(shader);
        drop(device);
        assert!(state.upgrade().is_some());
        assert_eq!(retained.evaluate(&[]).unwrap(), vec![[1.0; 4]]);
        drop(retained);
        assert!(state.upgrade().is_none());
    }
}
