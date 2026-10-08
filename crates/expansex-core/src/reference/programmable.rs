use super::{edge, top_left};
use crate::{
    DrawTriangles, GraphicsPipeline, IndexFormat, RenderExecutionError, Shader,
    ShaderExecutionError,
};

#[derive(Clone, Copy, Default)]
struct Vertex {
    clip: [f64; 4],
    varyings: [[f64; 4]; 16],
}

pub(super) struct PreparedDraw {
    vertices: Vec<Vertex>,
    pipeline: GraphicsPipeline,
}

pub(super) fn prepare(
    draw: &DrawTriangles,
    pipeline: &GraphicsPipeline,
) -> Result<PreparedDraw, RenderExecutionError> {
    let range = draw.vertices();
    let count = draw
        .indexed()
        .map_or((range.end - range.start) as usize, |indexed| {
            let indices = indexed.indices();
            (indices.end - indices.start) as usize
        });
    let stride = pipeline.vertex_stride();
    let mut vertices = Vec::new();
    vertices
        .try_reserve_exact(count)
        .map_err(|_| RenderExecutionError::AllocationFailed)?;
    if let Some(indexed) = draw.indexed().filter(|_| count != 0) {
        let indices = indexed.indices();
        let element_size = indexed.format().size();
        let start = (indexed.offset() + u64::from(indices.start) * element_size) as usize;
        let end = (indexed.offset() + u64::from(indices.end) * element_size) as usize;
        let capacity = (draw.vertex_buffer().size() - draw.vertex_offset()) / stride;
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(count)
            .map_err(|_| RenderExecutionError::AllocationFailed)?;
        // Release index storage before acquiring vertex storage. This matters
        // when both roles alias one allocation. The device gate preserves bytes.
        indexed
            .buffer()
            .with_bytes(start..end, |bytes| {
                for (position, record) in bytes.chunks_exact(element_size as usize).enumerate() {
                    let value = match indexed.format() {
                        IndexFormat::Uint16 => {
                            u32::from(u16::from_le_bytes([record[0], record[1]]))
                        }
                        IndexFormat::Uint32 => {
                            u32::from_le_bytes([record[0], record[1], record[2], record[3]])
                        }
                    };
                    let effective = i64::from(value) + i64::from(indexed.base_vertex());
                    let resolved = u32::try_from(effective)
                        .ok()
                        .filter(|&vertex| u64::from(vertex) < capacity)
                        .ok_or_else(|| RenderExecutionError::InvalidVertexIndex {
                            index_position: indices.start + position as u32,
                            index_value: value,
                            base_vertex: indexed.base_vertex(),
                            vertex_label: draw.vertex_buffer().label().to_owned(),
                            available_vertices: capacity,
                        })?;
                    selected.push(resolved);
                }
                Ok(())
            })
            .map_err(|_| RenderExecutionError::IndexStoragePoisoned {
                label: indexed.buffer().label().to_owned(),
            })??;
        draw.vertex_buffer()
            .with_bytes(
                draw.vertex_offset() as usize..draw.vertex_buffer().size() as usize,
                |bytes| {
                    for &vertex_index in &selected {
                        // Checked full-record capacity above also proves these usize bounds.
                        let start = (u64::from(vertex_index) * stride) as usize;
                        vertices.push(decode_vertex(
                            &bytes[start..start + stride as usize],
                            pipeline,
                            vertex_index,
                        )?);
                    }
                    Ok(())
                },
            )
            .map_err(|_| RenderExecutionError::VertexStoragePoisoned {
                label: draw.vertex_buffer().label().to_owned(),
            })??;
    } else if count != 0 {
        // Recording proved offset + end*stride fits buffer capacity (and usize).
        let start = (draw.vertex_offset() + u64::from(range.start) * stride) as usize;
        let end = (draw.vertex_offset() + u64::from(range.end) * stride) as usize;
        draw.vertex_buffer()
            .with_bytes(start..end, |bytes| {
                for (index, record) in bytes.chunks_exact(stride as usize).enumerate() {
                    let vertex_index = range.start + index as u32;
                    vertices.push(decode_vertex(record, pipeline, vertex_index)?);
                }
                Ok(())
            })
            .map_err(|_| RenderExecutionError::VertexStoragePoisoned {
                label: draw.vertex_buffer().label().to_owned(),
            })??;
    }
    Ok(PreparedDraw {
        vertices,
        pipeline: pipeline.clone(),
    })
}

fn decode_vertex(
    record: &[u8],
    pipeline: &GraphicsPipeline,
    vertex_index: u32,
) -> Result<Vertex, RenderExecutionError> {
    let mut inputs = [[0.0; 4]; 16];
    for attribute in pipeline.vertex_attributes() {
        let mut value = [0.0, 0.0, 0.0, 1.0];
        let begin = attribute.offset as usize;
        let end = begin + attribute.format.size() as usize;
        for (lane, &bytes) in record[begin..end].as_chunks::<4>().0.iter().enumerate() {
            value[lane] = f32::from_le_bytes(bytes);
        }
        inputs[attribute.location as usize] = value;
    }
    let mut outputs = [[0.0; 4]; 17];
    shade(
        pipeline.vertex_shader(),
        u64::from(vertex_index),
        &inputs[..pipeline.vertex_shader().input_count() as usize],
        &mut outputs,
    )?;
    if outputs[0][3] <= 0.0 {
        return Err(RenderExecutionError::InvalidClipW { vertex_index });
    }
    let mut vertex = Vertex {
        clip: outputs[0].map(f64::from),
        ..Vertex::default()
    };
    for (varying, value) in vertex
        .varyings
        .iter_mut()
        .zip(&outputs[1..pipeline.vertex_shader().outputs().len()])
    {
        *varying = value.map(f64::from);
    }
    Ok(vertex)
}

fn shade(
    shader: &Shader,
    invocation: u64,
    inputs: &[[f32; 4]],
    outputs: &mut [[f32; 4]],
) -> Result<(), RenderExecutionError> {
    shader
        .evaluate_into(inputs, outputs)
        .map_err(|error| match error {
            ShaderExecutionError::WrongInputCount { expected, actual } => {
                RenderExecutionError::ShaderInputCount {
                    stage: shader.stage(),
                    label: shader.label().to_owned(),
                    invocation,
                    expected,
                    actual,
                }
            }
            ShaderExecutionError::NonFiniteInput { location } => {
                RenderExecutionError::NonFiniteShaderInput {
                    stage: shader.stage(),
                    label: shader.label().to_owned(),
                    invocation,
                    location,
                }
            }
            ShaderExecutionError::NonFiniteResult { instruction } => {
                RenderExecutionError::NonFiniteShaderResult {
                    stage: shader.stage(),
                    label: shader.label().to_owned(),
                    invocation,
                    instruction,
                }
            }
            ShaderExecutionError::AllocationFailed { .. } => RenderExecutionError::AllocationFailed,
        })
}

pub(super) fn render(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    draw: &PreparedDraw,
) -> Result<(), RenderExecutionError> {
    for &triangle in draw.vertices.as_chunks::<3>().0 {
        clipped_triangle(pixels, width, height, triangle, &draw.pipeline)?;
    }
    Ok(())
}

fn clipped_triangle(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    triangle: [Vertex; 3],
    pipeline: &GraphicsPipeline,
) -> Result<(), RenderExecutionError> {
    // Six planes can add at most six vertices to a clipped convex triangle.
    let mut polygon = [Vertex::default(); 12];
    polygon[..3].copy_from_slice(&triangle);
    let mut count = 3;
    let varying_count = pipeline.fragment_shader().input_count() as usize;
    for (axis, sign) in [
        (0, 1.0),
        (0, -1.0),
        (1, 1.0),
        (1, -1.0),
        (2, 1.0),
        (2, -1.0),
    ] {
        if count == 0 {
            return Ok(());
        }
        let mut output = [Vertex::default(); 12];
        let mut output_count = 0;
        let distance = |vertex: Vertex| vertex.clip[3] + sign * vertex.clip[axis];
        let mut previous = polygon[count - 1];
        let mut previous_distance = distance(previous);
        for &current in &polygon[..count] {
            let current_distance = distance(current);
            if (previous_distance >= 0.0) != (current_distance >= 0.0) {
                let t = previous_distance / (previous_distance - current_distance);
                let mut intersection = Vertex::default();
                for lane in 0..4 {
                    intersection.clip[lane] =
                        previous.clip[lane] * (1.0 - t) + current.clip[lane] * t;
                }
                intersection.clip[axis] = -sign * intersection.clip[3];
                for varying in 0..varying_count {
                    for lane in 0..4 {
                        let a = previous.varyings[varying][lane];
                        let b = current.varyings[varying][lane];
                        intersection.varyings[varying][lane] =
                            (a * (1.0 - t) + b * t).clamp(a.min(b), a.max(b));
                    }
                }
                output[output_count] = intersection;
                output_count += 1;
            }
            if current_distance >= 0.0 {
                output[output_count] = current;
                output_count += 1;
            }
            previous = current;
            previous_distance = current_distance;
        }
        polygon = output;
        count = output_count;
    }
    if count < 3 {
        return Ok(());
    }
    for index in 1..count - 1 {
        rasterize(
            pixels,
            width,
            height,
            [polygon[0], polygon[index], polygon[index + 1]],
            pipeline,
        )?;
    }
    Ok(())
}

fn rasterize(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    mut triangle: [Vertex; 3],
    pipeline: &GraphicsPipeline,
) -> Result<(), RenderExecutionError> {
    let screen = |vertex: Vertex| {
        let x = (vertex.clip[0] / vertex.clip[3]).clamp(-1.0, 1.0);
        let y = (vertex.clip[1] / vertex.clip[3]).clamp(-1.0, 1.0);
        [
            ((x + 1.0) * 0.5 * f64::from(width) * 256.0).round() as i64,
            ((1.0 - y) * 0.5 * f64::from(height) * 256.0).round() as i64,
        ]
    };
    let mut points = triangle.map(screen);
    let mut area = edge(points[0], points[1], points[2]);
    if area == 0 {
        return Ok(());
    }
    if area < 0 {
        points.swap(1, 2);
        triangle.swap(1, 2);
        area = -area;
    }
    let min_x = points.iter().map(|p| p[0]).min().unwrap_or(0) / 256;
    let min_y = points.iter().map(|p| p[1]).min().unwrap_or(0) / 256;
    let max_x =
        ((points.iter().map(|p| p[0]).max().unwrap_or(0) + 255) / 256).min(i64::from(width));
    let max_y =
        ((points.iter().map(|p| p[1]).max().unwrap_or(0) + 255) / 256).min(i64::from(height));
    let edges = [
        (points[1], points[2]),
        (points[2], points[0]),
        (points[0], points[1]),
    ];
    let shader = pipeline.fragment_shader();
    let varying_count = shader.input_count() as usize;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let sample = [x * 256 + 128, y * 256 + 128];
            let values = edges.map(|(a, b)| edge(a, b, sample));
            if !values
                .iter()
                .zip(edges)
                .all(|(&value, (a, b))| value > 0 || (value == 0 && top_left(a, b)))
            {
                continue;
            }
            let weights: [f64; 3] = std::array::from_fn(|index| {
                values[index] as f64 / area as f64 / triangle[index].clip[3]
            });
            let denominator: f64 = weights.iter().sum();
            let mut inputs = [[0.0_f32; 4]; 16];
            for (varying, input) in inputs.iter_mut().enumerate().take(varying_count) {
                for (lane, component) in input.iter_mut().enumerate() {
                    let values = triangle.map(|v| v.varyings[varying][lane]);
                    let numerator: f64 = weights
                        .iter()
                        .zip(values)
                        .map(|(weight, value)| weight * value)
                        .sum();
                    let min = values[0].min(values[1]).min(values[2]);
                    let max = values[0].max(values[1]).max(values[2]);
                    // Bound tiny rounding overshoot to avoid manufacturing infinity
                    // from a convex combination of finite f32 values.
                    *component = (numerator / denominator).clamp(min, max) as f32;
                }
            }
            let invocation = y as u64 * u64::from(width) + x as u64;
            let mut output = [[0.0; 4]; 1];
            shade(shader, invocation, &inputs[..varying_count], &mut output)?;
            pixels[invocation as usize] =
                output[0].map(|lane| (lane.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    Ok(())
}
