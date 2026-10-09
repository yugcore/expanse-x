use crate::{fail, Interface, Reflection, Result, ShaderModule, UniformBinding};
use ex::{CoreProfile, ShaderInstruction as Op, ShaderStage};
use std::collections::{BTreeMap, BTreeSet};

// Numeric encodings from the Khronos SPIR-V 1.0 grammar. spirv-val verifies
// standard semantics before this narrower, fail-closed lowering boundary.
#[derive(Clone, Debug)]
enum Type {
    Void,
    Float,
    Int,
    Vector(u32, u32),
    Array,
    Struct(Vec<u32>),
    Pointer(u32, u32),
    Function,
}
#[derive(Clone, Debug)]
enum Constant {
    Float(f32),
    Int(u32),
    Vector([f32; 4]),
}
#[derive(Clone, Copy, Debug)]
enum Pointer {
    Input(u32),
    Output(u32),
    Position,
    PositionBlock,
    UniformBlock(u32),
    Uniform(u32, u32),
    Local(u32),
    Component(u32, u8),
    UniformComponent(u32, u32, u8),
    PositionComponent(u8),
}
struct Instruction<'a> {
    opcode: u16,
    words: &'a [u32],
    offset: usize,
}
struct Module {
    types: BTreeMap<u32, Type>,
    constants: BTreeMap<u32, (u32, Constant)>,
    names: BTreeMap<u32, String>,
    decorations: BTreeMap<u32, BTreeMap<u32, Vec<u32>>>,
    members: BTreeMap<(u32, u32, u32), Vec<u32>>,
    variables: Vec<(u32, u32, u32)>,
    pointers: BTreeMap<u32, Pointer>,
    values: BTreeMap<u32, (u32, u32)>,
    locals: BTreeMap<u32, (u32, u32)>,
    stores: BTreeMap<u32, u32>,
    ops: Vec<Op>,
    stage: Option<ShaderStage>,
    entry: Option<u32>,
    interfaces: BTreeSet<u32>,
    reflection: Option<Reflection>,
}

fn string(words: &[u32]) -> Result<(String, usize)> {
    let mut bytes = Vec::new();
    for (index, word) in words.iter().enumerate() {
        for byte in word.to_le_bytes() {
            if byte == 0 {
                return Ok((
                    String::from_utf8(bytes).map_err(|_| fail("non-UTF-8 SPIR-V string"))?,
                    index + 1,
                ));
            }
            bytes.push(byte);
        }
    }
    Err(fail("unterminated SPIR-V string"))
}
fn framing(bytes: &[u8]) -> Result<Vec<u32>> {
    if bytes.len() < 20 || !bytes.len().is_multiple_of(4) || bytes.len() > 1024 * 1024 {
        return Err(fail(
            "SPIR-V must have a bounded five-word header and complete words",
        ));
    }
    let words: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    if words[0] != 0x0723_0203
        || words[1] != 0x0001_0000
        || words[3] == 0
        || words[3] > 65536
        || words[4] != 0
    {
        return Err(fail(
            "EX-GLSL-1 requires little-endian SPIR-V 1.0, ID bound 1..=65536 and schema zero",
        ));
    }
    Ok(words)
}
fn instructions(words: &[u32]) -> Result<Vec<Instruction<'_>>> {
    let mut result = Vec::new();
    let mut offset = 5;
    while offset < words.len() {
        let count = (words[offset] >> 16) as usize;
        if count == 0 || count > words.len() - offset {
            return Err(fail(format!(
                "invalid SPIR-V instruction length at word {offset}"
            )));
        }
        result.push(Instruction {
            opcode: words[offset] as u16,
            words: &words[offset + 1..offset + count],
            offset,
        });
        offset += count;
    }
    Ok(result)
}
impl Module {
    fn new() -> Self {
        Self {
            types: BTreeMap::new(),
            constants: BTreeMap::new(),
            names: BTreeMap::new(),
            decorations: BTreeMap::new(),
            members: BTreeMap::new(),
            variables: Vec::new(),
            pointers: BTreeMap::new(),
            values: BTreeMap::new(),
            locals: BTreeMap::new(),
            stores: BTreeMap::new(),
            ops: Vec::new(),
            stage: None,
            entry: None,
            interfaces: BTreeSet::new(),
            reflection: None,
        }
    }
    fn width(&self, ty: u32) -> Result<usize> {
        match self.types.get(&ty) {
            Some(Type::Float) => Ok(1),
            Some(Type::Vector(component, 4))
                if matches!(self.types.get(component), Some(Type::Float)) =>
            {
                Ok(4)
            }
            _ => Err(fail("only float32 and vec4 arithmetic is supported")),
        }
    }
    fn vec4(&self, ty: u32) -> Result<()> {
        if self.width(ty)? != 4 {
            return Err(fail("interface/uniform value must be vec4"));
        }
        Ok(())
    }
    fn decoration(&self, id: u32, decoration: u32) -> Option<&[u32]> {
        self.decorations
            .get(&id)?
            .get(&decoration)
            .map(Vec::as_slice)
    }
    fn number(&self, id: u32, decoration: u32) -> Result<u32> {
        match self.decoration(id, decoration) {
            Some([value]) => Ok(*value),
            _ => Err(fail(format!("ID {id} requires decoration {decoration}"))),
        }
    }
    fn name(&self, id: u32) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("id_{id}"))
    }
    fn emit(&mut self, op: Op) -> Result<u32> {
        if self.ops.len() >= CoreProfile::Preview1.limits().max_shader_instructions as usize {
            return Err(fail("lowering exceeds EX's 256 instruction limit"));
        }
        let register = self.ops.len() as u32;
        self.ops.push(op);
        Ok(register)
    }
    fn value(&mut self, id: u32) -> Result<(u32, u32)> {
        if let Some(value) = self.values.get(&id) {
            return Ok(*value);
        }
        let (ty, constant) = self
            .constants
            .get(&id)
            .cloned()
            .ok_or_else(|| fail(format!("value ID {id} is unavailable")))?;
        let lanes = match constant {
            Constant::Float(value) => [value; 4],
            Constant::Vector(value) => value,
            Constant::Int(_) => {
                return Err(fail(
                    "integer values are only supported as static member indices",
                ))
            }
        };
        let reg = self.emit(Op::Constant(lanes))?;
        self.values.insert(id, (ty, reg));
        Ok((ty, reg))
    }
    fn static_index(&self, id: u32) -> Result<u32> {
        match self.constants.get(&id) {
            Some((_, Constant::Int(value))) => Ok(*value),
            _ => Err(fail(
                "access chains require a single constant integer member index",
            )),
        }
    }
    fn load_pointer(&mut self, pointer: Pointer, width: usize) -> Result<u32> {
        match pointer {
            Pointer::Input(location) if width == 4 => self.emit(Op::Input { location }),
            Pointer::Uniform(binding, element) if width == 4 => {
                self.emit(Op::LoadUniform { binding, element })
            }
            Pointer::Local(id) => {
                let (ty, reg) = *self
                    .locals
                    .get(&id)
                    .ok_or_else(|| fail("read of uninitialized local"))?;
                if self.width(ty)? != width {
                    return Err(fail("local load type mismatch"));
                }
                Ok(reg)
            }
            Pointer::Output(location) if width == 4 => self
                .stores
                .get(&location)
                .copied()
                .ok_or_else(|| fail("read of unwritten output")),
            Pointer::Position if width == 4 => self
                .stores
                .get(&u32::MAX)
                .copied()
                .ok_or_else(|| fail("read of unwritten position")),
            Pointer::Component(base, component) if width == 1 => {
                let base = *self
                    .pointers
                    .get(&base)
                    .ok_or_else(|| fail("unknown component base"))?;
                if matches!(base, Pointer::Component(_, _)) {
                    return Err(fail("nested component pointers are unsupported"));
                }
                let value = self.load_pointer(base, 4)?;
                self.emit(Op::Swizzle {
                    value,
                    components: [component; 4],
                })
            }
            Pointer::UniformComponent(binding, element, component) if width == 1 => {
                let value = self.emit(Op::LoadUniform { binding, element })?;
                self.emit(Op::Swizzle {
                    value,
                    components: [component; 4],
                })
            }
            Pointer::PositionComponent(component) if width == 1 => {
                let value = self.load_pointer(Pointer::Position, 4)?;
                self.emit(Op::Swizzle {
                    value,
                    components: [component; 4],
                })
            }
            _ => Err(fail("unsupported load type or block/component load")),
        }
    }
    fn global(&mut self, instruction: &Instruction<'_>) -> Result<()> {
        let a = instruction.words;
        match (instruction.opcode, a) {
            (0, []) | (3, [_, _, ..]) | (6, [_, _, ..]) | (8, [_, _, _]) | (317, []) => {}
            (5, [id, words @ ..]) => {
                self.names.insert(*id, string(words)?.0);
            }
            (7, [_, words @ ..]) => {
                string(words)?;
            }
            (11, [_, words @ ..]) if string(words)?.0 == "GLSL.std.450" => {}
            (14, [0, 1]) | (17, [1]) => {}
            (15, [model, entry, words @ ..]) => {
                if self.entry.is_some() {
                    return Err(fail("only one entry point is supported"));
                }
                let (name, consumed) = string(words)?;
                if name != "main" {
                    return Err(fail("entry point must be main"));
                }
                self.stage = Some(match model {
                    0 => ShaderStage::Vertex,
                    4 => ShaderStage::Fragment,
                    _ => return Err(fail("only vertex and fragment import is supported")),
                });
                self.entry = Some(*entry);
                self.interfaces.extend(words[consumed..].iter().copied());
            }
            (16, [entry, 7])
                if Some(*entry) == self.entry && self.stage == Some(ShaderStage::Fragment) => {}
            (71, [id, decoration, values @ ..]) if matches!(decoration, 2 | 11 | 30 | 33 | 34) => {
                self.decorations
                    .entry(*id)
                    .or_default()
                    .insert(*decoration, values.to_vec());
            }
            (72, [ty, member, decoration, values @ ..]) if matches!(decoration, 11 | 35) => {
                self.members
                    .insert((*ty, *member, *decoration), values.to_vec());
            }
            (19, [id]) => {
                self.types.insert(*id, Type::Void);
            }
            (21, [id, 32, 0 | 1]) => {
                self.types.insert(*id, Type::Int);
            }
            (22, [id, 32]) => {
                self.types.insert(*id, Type::Float);
            }
            (23, [id, component, count]) if *count == 4 => {
                self.types.insert(*id, Type::Vector(*component, *count));
            }
            (28, [id, _, _]) => {
                self.types.insert(*id, Type::Array);
            }
            (30, [id, members @ ..]) => {
                self.types.insert(*id, Type::Struct(members.to_vec()));
            }
            (32, [id, storage, ty]) => {
                self.types.insert(*id, Type::Pointer(*storage, *ty));
            }
            (33, [id, _]) => {
                self.types.insert(*id, Type::Function);
            }
            (43, [ty, id, bits]) => {
                let constant = match self.types.get(ty) {
                    Some(Type::Float) => {
                        let value = f32::from_bits(*bits);
                        if !value.is_finite() {
                            return Err(fail("non-finite shader constant"));
                        }
                        Constant::Float(value)
                    }
                    Some(Type::Int) => Constant::Int(*bits),
                    _ => return Err(fail("unsupported constant type")),
                };
                self.constants.insert(*id, (*ty, constant));
            }
            (44, [ty, id, x, y, z, w]) => {
                self.vec4(*ty)?;
                let mut lanes = [0.0; 4];
                for (lane, id) in lanes.iter_mut().zip([x, y, z, w]) {
                    match self.constants.get(id) {
                        Some((_, Constant::Float(v))) => *lane = *v,
                        _ => return Err(fail("vec4 constant requires four float constants")),
                    }
                }
                self.constants.insert(*id, (*ty, Constant::Vector(lanes)));
            }
            (59, [ty, id, storage]) if matches!(storage, 1..=3) => {
                self.variables.push((*ty, *id, *storage));
            }
            _ => {
                return Err(fail(
                    "unsupported module instruction, type, extension or decoration",
                ))
            }
        }
        Ok(())
    }
    fn prepare(&mut self) -> Result<()> {
        let stage = self.stage.ok_or_else(|| fail("missing entry point"))?;
        let mut reflection = Reflection {
            stage,
            inputs: Vec::new(),
            outputs: Vec::new(),
            uniforms: Vec::new(),
            instruction_count: 0,
        };
        let mut position = false;
        for (pointer, id, storage) in self.variables.clone() {
            let ty = match self.types.get(&pointer) {
                Some(Type::Pointer(class, ty)) if *class == storage => *ty,
                _ => return Err(fail("variable pointer/storage mismatch")),
            };
            let ptr = match storage {
                1 | 3 => {
                    if !self.interfaces.contains(&id) {
                        return Err(fail("input/output variable missing from main interface"));
                    }
                    if let Some(Type::Struct(members)) = self.types.get(&ty) {
                        // gl_PerVertex's unused standard members are declarations only.
                        if storage != 3
                            || stage != ShaderStage::Vertex
                            || self.decoration(ty, 2) != Some(&[])
                            || self.members.get(&(ty, 0, 11)).map(Vec::as_slice) != Some(&[0])
                        {
                            return Err(fail(
                                "only the vertex built-in position block is supported",
                            ));
                        }
                        self.vec4(
                            *members
                                .first()
                                .ok_or_else(|| fail("empty position block"))?,
                        )?;
                        if position {
                            return Err(fail("duplicate vertex position"));
                        }
                        position = true;
                        Pointer::PositionBlock
                    } else if self.decoration(id, 11) == Some(&[0])
                        && storage == 3
                        && stage == ShaderStage::Vertex
                    {
                        self.vec4(ty)?;
                        if position {
                            return Err(fail("duplicate vertex position"));
                        }
                        position = true;
                        Pointer::Position
                    } else {
                        self.vec4(ty)?;
                        if self.decoration(id, 11).is_some() {
                            return Err(fail("unsupported built-in interface"));
                        }
                        let location = self.number(id, 30)?;
                        let interface = Interface {
                            name: self.name(id),
                            location,
                        };
                        if storage == 1 {
                            reflection.inputs.push(interface);
                            Pointer::Input(location)
                        } else {
                            reflection.outputs.push(interface);
                            Pointer::Output(location)
                        }
                    }
                }
                2 => {
                    let binding = self.number(id, 33)?;
                    if self.number(id, 34)? != 0
                        || binding >= CoreProfile::Preview1.limits().binding_slots
                    {
                        return Err(fail(
                            "uniforms require descriptor set zero and binding 0..7",
                        ));
                    }
                    let members = match self.types.get(&ty) {
                        Some(Type::Struct(members)) if self.decoration(ty, 2) == Some(&[]) => {
                            members
                        }
                        _ => return Err(fail("uniform must be a vec4 block")),
                    };
                    if members.is_empty() || members.len() > 256 {
                        return Err(fail("uniform block requires 1..256 vec4 members"));
                    }
                    for (index, member) in members.iter().enumerate() {
                        self.vec4(*member)?;
                        if self.members.get(&(ty, index as u32, 35)).map(Vec::as_slice)
                            != Some(&[(index * 16) as u32])
                        {
                            return Err(fail("uniform vec4 member offsets must be 0,16,32,..."));
                        }
                    }
                    reflection.uniforms.push(UniformBinding {
                        name: self.name(id),
                        binding,
                        minimum_bytes: members.len() as u64 * 16,
                    });
                    Pointer::UniformBlock(binding)
                }
                _ => return Err(fail("unsupported variable storage class")),
            };
            self.pointers.insert(id, ptr);
        }
        for values in [&mut reflection.inputs, &mut reflection.outputs] {
            values.sort_by_key(|value| value.location);
            if values
                .iter()
                .enumerate()
                .any(|(i, value)| value.location != i as u32)
            {
                return Err(fail(
                    "interface locations must be unique and consecutive from zero",
                ));
            }
        }
        if reflection.inputs.len() > 16 || reflection.outputs.len() > 16 {
            return Err(fail("interface exceeds EX's 16 user locations"));
        }
        if stage == ShaderStage::Vertex && !position {
            return Err(fail("vertex shader must declare gl_Position"));
        }
        if stage == ShaderStage::Fragment && reflection.outputs.len() != 1 {
            return Err(fail(
                "fragment shader must declare exactly one vec4 colour at location zero",
            ));
        }
        reflection.uniforms.sort_by_key(|u| u.binding);
        if reflection
            .uniforms
            .windows(2)
            .any(|w| w[0].binding == w[1].binding)
        {
            return Err(fail("duplicate uniform binding slot"));
        }
        self.reflection = Some(reflection);
        Ok(())
    }
    fn body(&mut self, instruction: &Instruction<'_>) -> Result<()> {
        let a = instruction.words;
        match (instruction.opcode, a) {
            (0, []) | (8, [_, _, _]) | (317, []) => {},
            (59, [pointer, id, 7]) => {
                let ty = match self.types.get(pointer) { Some(Type::Pointer(7, ty)) => *ty, _ => return Err(fail("invalid local pointer")) };
                self.width(ty)?;
                self.pointers.insert(*id, Pointer::Local(*id));
            },
            (65, [_, id, base, index]) => {
                let index = self.static_index(*index)?;
                let pointer = match self.pointers.get(base) {
                    Some(Pointer::PositionBlock) if index == 0 => Pointer::Position,
                    Some(Pointer::UniformBlock(binding)) => Pointer::Uniform(*binding, index),
                    Some(Pointer::Input(_) | Pointer::Output(_) | Pointer::Position | Pointer::Uniform(_, _) | Pointer::Local(_)) if index < 4 => Pointer::Component(*base, index as u8),
                    _ => return Err(fail("only static uniform members and gl_Position access are supported")),
                };
                self.pointers.insert(*id, pointer);
            },
            (65, [_, id, base, member, component]) => {
                let member = self.static_index(*member)?;
                let component = self.static_index(*component)?;
                if component >= 4 { return Err(fail("component index must be 0..3")); }
                let pointer = match self.pointers.get(base) {
                    Some(Pointer::UniformBlock(binding)) => Pointer::UniformComponent(*binding, member, component as u8),
                    Some(Pointer::PositionBlock) if member == 0 => Pointer::PositionComponent(component as u8),
                    _ => return Err(fail("only static uniform/position block components are supported")),
                };
                self.pointers.insert(*id, pointer);
            },
            (61, [ty, id, pointer]) => {
                self.width(*ty)?;
                let pointer = *self.pointers.get(pointer).ok_or_else(|| fail("unknown load pointer"))?;
                let register = self.load_pointer(pointer, self.width(*ty)?)?;
                self.values.insert(*id, (*ty, register));
            },
            (62, [pointer, value]) => {
                let (ty, reg) = self.value(*value)?;
                match self.pointers.get(pointer) {
                    Some(Pointer::Local(id)) => { self.locals.insert(*id, (ty, reg)); },
                    Some(Pointer::Output(location)) => { self.vec4(ty)?; self.stores.insert(*location, reg); },
                    Some(Pointer::Position) => { self.vec4(ty)?; self.stores.insert(u32::MAX, reg); },
                    _ => return Err(fail("unsupported store destination")),
                }
            },
            (129 | 133 | 142 | 148, [ty, id, left, right]) => {
                let (left_ty, left) = self.value(*left)?;
                let (right_ty, right) = self.value(*right)?;
                let output_width = self.width(*ty)?;
                let lw = self.width(left_ty)?;
                let rw = self.width(right_ty)?;
                let op = match instruction.opcode {
                    129 if lw == rw && lw == output_width => Op::Add { left, right },
                    133 if lw == rw && lw == output_width => Op::Multiply { left, right },
                    142 if lw == 4 && rw == 1 && output_width == 4 => Op::Multiply { left, right },
                    148 if lw == 4 && rw == 4 && output_width == 1 => Op::Dot4 { left, right },
                    _ => return Err(fail("arithmetic operand types are outside the supported subset")),
                };
                let reg = self.emit(op)?;
                self.values.insert(*id, (*ty, reg));
            },
            (81, [ty, id, value, component]) if *component < 4 => {
                let (source_ty, value) = self.value(*value)?;
                self.vec4(source_ty)?;
                if self.width(*ty)? != 1 { return Err(fail("component extraction must return float")); }
                let reg = self.emit(Op::Swizzle { value, components: [*component as u8; 4] })?;
                self.values.insert(*id, (*ty, reg));
            },
            (80, [ty, id, x, y, z, w]) => {
                self.vec4(*ty)?;
                let mut pieces = Vec::new();
                for (lane, scalar) in [x, y, z, w].into_iter().enumerate() {
                    let (scalar_ty, value) = self.value(*scalar)?;
                    if self.width(scalar_ty)? != 1 { return Err(fail("vec4 construction requires four scalars")); }
                    let mut mask = [0.0; 4]; mask[lane] = 1.0;
                    let mask = self.emit(Op::Constant(mask))?;
                    pieces.push(self.emit(Op::Multiply { left: value, right: mask })?);
                }
                let mut reg = pieces[0];
                for right in &pieces[1..] { reg = self.emit(Op::Add { left: reg, right: *right })?; }
                self.values.insert(*id, (*ty, reg));
            },
            (79, [ty, id, first, second, x, y, z, w]) => {
                self.vec4(*ty)?;
                let (first_ty, first) = self.value(*first)?;
                let (second_ty, second) = self.value(*second)?;
                self.vec4(first_ty)?; self.vec4(second_ty)?;
                let selectors = [*x, *y, *z, *w];
                if selectors.iter().any(|s| *s >= 8) { return Err(fail("undefined/out-of-range swizzle component")); }
                let reg = if selectors.iter().all(|s| *s < 4) {
                    self.emit(Op::Swizzle { value: first, components: selectors.map(|s| s as u8) })?
                } else if selectors.iter().all(|s| *s >= 4) {
                    self.emit(Op::Swizzle { value: second, components: selectors.map(|s| (s - 4) as u8) })?
                } else {
                    let l = self.emit(Op::Swizzle { value: first, components: selectors.map(|s| (s % 4) as u8) })?;
                    let r = self.emit(Op::Swizzle { value: second, components: selectors.map(|s| (s % 4) as u8) })?;
                    let lm = self.emit(Op::Constant(selectors.map(|s| if s < 4 { 1.0 } else { 0.0 })))?;
                    let rm = self.emit(Op::Constant(selectors.map(|s| if s >= 4 { 1.0 } else { 0.0 })))?;
                    let l = self.emit(Op::Multiply { left: l, right: lm })?;
                    let r = self.emit(Op::Multiply { left: r, right: rm })?;
                    self.emit(Op::Add { left: l, right: r })?
                };
                self.values.insert(*id, (*ty, reg));
            },
            (83, [ty, id, value]) => {
                let (source_ty, reg) = self.value(*value)?;
                if source_ty != *ty { return Err(fail("copy type mismatch")); }
                self.values.insert(*id, (*ty, reg));
            },
            _ => return Err(fail("unsupported executable instruction (control flow, functions, textures and other operations are not imported)")),
        }
        Ok(())
    }
}

pub(crate) fn lower(bytes: &[u8], label: &str) -> Result<ShaderModule> {
    let words = framing(bytes)?;
    let decoded = instructions(&words)?;
    let mut module = Module::new();
    let mut function = false;
    let mut began = false;
    let mut block = false;
    let mut returned = false;
    for instruction in decoded {
        let result = (|| {
            match (instruction.opcode, instruction.words) {
                (54, [ty, id, 0, _])
                    if !began
                        && Some(*id) == module.entry
                        && matches!(module.types.get(ty), Some(Type::Void)) =>
                {
                    module.prepare()?;
                    function = true;
                    began = true;
                }
                (248, [_]) if function && !block => {
                    block = true;
                }
                (253, []) if function && block && !returned => {
                    returned = true;
                }
                (56, []) if function && returned => {
                    function = false;
                }
                _ if function && block && !returned => module.body(&instruction)?,
                _ if !began => module.global(&instruction)?,
                _ => {
                    return Err(fail(
                        "only one straight-line main function and one return are supported",
                    ))
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            return Err(fail(format!(
                "EX-GLSL-1: opcode {} at SPIR-V word {}: {error}",
                instruction.opcode, instruction.offset
            )));
        }
    }
    if !began || function || !returned {
        return Err(fail("missing or incomplete main function"));
    }
    let mut reflection = module
        .reflection
        .ok_or_else(|| fail("missing reflection"))?;
    let mut outputs = Vec::new();
    if reflection.stage == ShaderStage::Vertex {
        outputs.push(
            *module
                .stores
                .get(&u32::MAX)
                .ok_or_else(|| fail("gl_Position is not written"))?,
        );
    }
    for output in &reflection.outputs {
        outputs.push(*module.stores.get(&output.location).ok_or_else(|| {
            fail(format!(
                "output location {} is not written",
                output.location
            ))
        })?);
    }
    reflection.instruction_count = module.ops.len();
    let result = ShaderModule {
        label: label.to_owned(),
        reflection,
        instructions: module.ops,
        outputs,
    };
    result
        .descriptor()
        .validate()
        .map_err(|e| fail(format!("lowered EX shader validation failed: {e}")))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_rejects_truncation_wrong_version_and_zero_length() {
        assert!(lower(&[], "bad").is_err());
        for tail in [0u32, 0x0002_0011] {
            let bytes: Vec<_> = [0x0723_0203u32, 0x0001_0000, 0, 1, 0, tail]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect();
            assert!(lower(&bytes, "bad").is_err());
        }
    }
    #[test]
    fn json_escapes_names_without_losing_unicode() {
        let r = Reflection {
            stage: ShaderStage::Fragment,
            inputs: vec![Interface {
                name: "\"\\\nλ".into(),
                location: 0,
            }],
            outputs: vec![],
            uniforms: vec![],
            instruction_count: 1,
        };
        assert!(r.to_json().contains("\\\"\\\\\\u000aλ"));
    }
}
