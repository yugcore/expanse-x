//! Working CPU executor. Queue policy owns the device gate; this module owns
//! byte-copy interpretation, vertex fetch, clipping, and pixel rasterization.
use crate::{Command, CopyBufferRole, QueueSubmitError, RenderCommands, RenderExecutionError};
mod programmable;

pub(crate) fn execute(command_index: usize, command: &Command) -> Result<(), QueueSubmitError> {
    match command {
        Command::CopyBuffer(copy) => copy
            .source()
            .copy_to(
                copy.source_offset(),
                copy.destination(),
                copy.destination_offset(),
                copy.size(),
            )
            .map_err(|role| QueueSubmitError::StoragePoisoned {
                command_index,
                role,
                label: match role {
                    CopyBufferRole::Source => copy.source().label(),
                    CopyBufferRole::Destination => copy.destination().label(),
                }
                .to_owned(),
            }),
        Command::RenderPass(pass) => {
            render(pass).map_err(|source| QueueSubmitError::RenderFailed {
                command_index,
                source,
            })
        }
    }
}

struct FixedDraw {
    positions: Vec<[f64; 2]>,
    colour: [u8; 4],
}

enum PreparedDraw {
    Fixed(FixedDraw),
    Programmable(programmable::PreparedDraw),
}

fn render(pass: &RenderCommands) -> Result<(), RenderExecutionError> {
    // Preflight vertices, then shade into a temporary image. No allocation or
    // expected fallible operation occurs once target commit begins.
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(pass.draws().len())
        .map_err(|_| RenderExecutionError::AllocationFailed)?;
    for draw in pass.draws() {
        if let Some(pipeline) = draw.pipeline() {
            prepared.push(PreparedDraw::Programmable(programmable::prepare(
                draw, pipeline,
            )?));
            continue;
        }
        let vertices = draw.vertices();
        let count = (vertices.end - vertices.start) as usize;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(count)
            .map_err(|_| RenderExecutionError::AllocationFailed)?;
        if count != 0 {
            let start = (draw.vertex_offset() + u64::from(vertices.start) * 8) as usize;
            let end = start + count * 8;
            draw.vertex_buffer()
                .with_bytes(start..end, |bytes| {
                    for (index, vertex) in bytes.as_chunks::<8>().0.iter().enumerate() {
                        let x = f32::from_le_bytes([vertex[0], vertex[1], vertex[2], vertex[3]]);
                        let y = f32::from_le_bytes([vertex[4], vertex[5], vertex[6], vertex[7]]);
                        if !x.is_finite() || !y.is_finite() {
                            return Err(RenderExecutionError::NonFiniteVertex {
                                label: draw.vertex_buffer().label().to_owned(),
                                vertex_index: vertices.start + index as u32,
                            });
                        }
                        positions.push([f64::from(x), f64::from(y)]);
                    }
                    Ok(())
                })
                .map_err(|_| RenderExecutionError::VertexStoragePoisoned {
                    label: draw.vertex_buffer().label().to_owned(),
                })??;
        }
        prepared.push(PreparedDraw::Fixed(FixedDraw {
            positions,
            colour: draw.colour(),
        }));
    }
    let target = pass.target();
    target
        .with_pixels_mut(|pixels| {
            // Late fragment failures must preserve the entire target, including
            // clear and earlier draws in this scope. Commit only on full success.
            let mut staged = Vec::new();
            staged
                .try_reserve_exact(pixels.len())
                .map_err(|_| RenderExecutionError::AllocationFailed)?;
            staged.extend_from_slice(pixels);
            if let Some(colour) = pass.clear_colour() {
                staged.fill(colour);
            }
            for draw in prepared {
                match draw {
                    PreparedDraw::Programmable(draw) => {
                        programmable::render(&mut staged, target.width(), target.height(), &draw)?
                    }
                    PreparedDraw::Fixed(draw) => {
                        for triangle in draw.positions.as_chunks::<3>().0 {
                            clipped_triangle(
                                &mut staged,
                                target.width(),
                                target.height(),
                                [triangle[0], triangle[1], triangle[2]],
                                draw.colour,
                            );
                        }
                    }
                }
            }
            pixels.copy_from_slice(&staged);
            Ok(())
        })
        .map_err(|_| RenderExecutionError::FramebufferStoragePoisoned {
            label: target.label().to_owned(),
        })?
}

fn clipped_triangle(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    triangle: [[f64; 2]; 3],
    colour: [u8; 4],
) {
    // A convex triangle clipped by four half-planes has at most seven vertices.
    // Fixed arrays avoid allocating after render preflight.
    let mut polygon = [[0.0; 2]; 8];
    polygon[..3].copy_from_slice(&triangle);
    let mut count = 3;
    for (axis, bound, lower) in [
        (0, -1.0, true),
        (0, 1.0, false),
        (1, -1.0, true),
        (1, 1.0, false),
    ] {
        if count == 0 {
            return;
        }
        let mut output = [[0.0; 2]; 8];
        let mut output_count = 0;
        let inside = |point: [f64; 2]| {
            if lower {
                point[axis] >= bound
            } else {
                point[axis] <= bound
            }
        };
        let mut previous = polygon[count - 1];
        for &current in &polygon[..count] {
            let previous_inside = inside(previous);
            let current_inside = inside(current);
            if previous_inside != current_inside {
                let t = (bound - previous[axis]) / (current[axis] - previous[axis]);
                let mut intersection = previous;
                intersection[axis] = bound;
                let other = 1 - axis;
                intersection[other] = previous[other] + (current[other] - previous[other]) * t;
                output[output_count] = intersection;
                output_count += 1;
            }
            if current_inside {
                output[output_count] = current;
                output_count += 1;
            }
            previous = current;
        }
        polygon = output;
        count = output_count;
    }
    if count < 3 {
        return;
    }
    let screen = |point: [f64; 2]| {
        [
            ((point[0].clamp(-1.0, 1.0) + 1.0) * 0.5 * f64::from(width) * 256.0).round() as i64,
            ((1.0 - point[1].clamp(-1.0, 1.0)) * 0.5 * f64::from(height) * 256.0).round() as i64,
        ]
    };
    for index in 1..count - 1 {
        rasterize(
            pixels,
            width,
            height,
            [
                screen(polygon[0]),
                screen(polygon[index]),
                screen(polygon[index + 1]),
            ],
            colour,
        );
    }
}

fn edge(a: [i64; 2], b: [i64; 2], p: [i64; 2]) -> i64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

fn top_left(a: [i64; 2], b: [i64; 2]) -> bool {
    b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])
}

fn rasterize(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    mut points: [[i64; 2]; 3],
    colour: [u8; 4],
) {
    let area = edge(points[0], points[1], points[2]);
    if area == 0 {
        return;
    }
    if area < 0 {
        points.swap(1, 2);
    }
    let min_x = points.iter().map(|p| p[0]).min().unwrap_or(0) / 256;
    let min_y = points.iter().map(|p| p[1]).min().unwrap_or(0) / 256;
    let max_x =
        ((points.iter().map(|p| p[0]).max().unwrap_or(0) + 255) / 256).min(i64::from(width));
    let max_y =
        ((points.iter().map(|p| p[1]).max().unwrap_or(0) + 255) / 256).min(i64::from(height));
    let edges = [
        (points[0], points[1]),
        (points[1], points[2]),
        (points[2], points[0]),
    ];
    for y in min_y..max_y {
        for x in min_x..max_x {
            let sample = [x * 256 + 128, y * 256 + 128];
            if edges.iter().all(|&(a, b)| {
                let value = edge(a, b, sample);
                value > 0 || (value == 0 && top_left(a, b))
            }) {
                pixels[y as usize * width as usize + x as usize] = colour;
            }
        }
    }
}
