//! Draws the 3D view on the GPU, inside an egui panel (ADR 0003): the
//! volumes of the model, lit, with a depth buffer.
//!
//! The triangles live in GPU memory and are sent again only when the model
//! changes; moving the camera only sends one matrix.

use std::sync::Arc;

use archibim_geometry::{Block, triangulate};
use archibim_model::Point;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};

/// Bits of the depth buffer the window must be opened with
/// (`NativeOptions::depth_buffer`).
pub const DEPTH_BITS: u8 = 32;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One vertex: position in millimetres, normal, colour from 0 to 1.
pub type Vertex = [f32; 9];

/// Vertical field of view, in radians.
const FOV: f32 = 0.8;

/// The triangles of a model, ready for the GPU.
#[derive(Debug, Default)]
pub struct Scene {
    /// Changes whenever the triangles change.
    pub generation: u64,
    /// Three vertices per triangle.
    pub vertices: Vec<Vertex>,
}

impl Scene {
    /// The triangles of `blocks`, each block in its colour.
    pub fn new(generation: u64, blocks: &[(&Block, [f32; 3])]) -> Self {
        let mut vertices = Vec::new();
        for (block, color) in blocks {
            add_block(&mut vertices, block, *color);
        }
        Self {
            generation,
            vertices,
        }
    }

    /// The centre and the radius of a sphere around the model.
    fn bounds(&self) -> Option<([f32; 3], f32)> {
        let first = self.vertices.first()?;
        let (mut lo, mut hi) = (
            [first[0], first[1], first[2]],
            [first[0], first[1], first[2]],
        );
        for v in &self.vertices {
            for i in 0..3 {
                lo[i] = lo[i].min(v[i]);
                hi[i] = hi[i].max(v[i]);
            }
        }
        let centre = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.0);
        Some((centre, length(sub(hi, lo)) / 2.0))
    }
}

fn add_block(out: &mut Vec<Vertex>, block: &Block, color: [f32; 3]) {
    let mut corners: Vec<(Point, f32, f32)> = block
        .outline
        .iter()
        .zip(&block.bottom)
        .zip(&block.top)
        .map(|((p, b), t)| (*p, *b as f32, *t as f32))
        .collect();
    let outline: Vec<Point> = corners.iter().map(|c| c.0).collect();
    if signed_area(&outline) < 0.0 {
        corners.reverse();
    }
    let outline: Vec<Point> = corners.iter().map(|c| c.0).collect();
    let at = |i: usize, top: bool| {
        let (p, z0, z1) = corners[i];
        [p.x as f32, p.y as f32, if top { z1 } else { z0 }]
    };
    let mut push = |p: [f32; 3], n: [f32; 3]| {
        out.push([
            p[0], p[1], p[2], n[0], n[1], n[2], color[0], color[1], color[2],
        ]);
    };
    for [a, b, c] in triangulate(&outline) {
        let face = |top: bool| {
            normalize(cross(
                sub(at(b, top), at(a, top)),
                sub(at(c, top), at(a, top)),
            ))
        };
        let up = face(true);
        for i in [a, b, c] {
            push(at(i, true), up);
        }
        let down = scale(face(false), -1.0);
        for i in [a, c, b] {
            push(at(i, false), down);
        }
    }
    let n = outline.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (outline[i], outline[j]);
        if corners[i].1 >= corners[i].2 && corners[j].1 >= corners[j].2 {
            continue;
        }
        let normal = normalize([(b.y - a.y) as f32, (a.x - b.x) as f32, 0.0]);
        for p in [
            at(i, false),
            at(j, false),
            at(j, true),
            at(i, false),
            at(j, true),
            at(i, true),
        ] {
            push(p, normal);
        }
    }
}

fn signed_area(outline: &[Point]) -> f64 {
    let n = outline.len();
    (0..n)
        .map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum()
}

/// An orbiting camera: it looks at `target` from `distance` away, turned by
/// `yaw` around the vertical and raised by `pitch`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The point looked at, in millimetres.
    pub target: [f32; 3],
    /// Turn around the vertical, in radians.
    pub yaw: f32,
    /// Angle above the horizontal, in radians.
    pub pitch: f32,
    /// Distance to the target, in millimetres; 0 until fitted.
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0; 3],
            yaw: -2.2,
            pitch: 0.5,
            distance: 0.0,
        }
    }
}

impl Camera {
    fn eye(&self) -> [f32; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        add(self.target, scale([cp * cy, cp * sy, sp], self.distance))
    }

    /// The screen's right and up directions in the model.
    pub fn right_up(&self) -> ([f32; 3], [f32; 3]) {
        let forward = normalize(sub(self.target, self.eye()));
        let right = normalize(cross(forward, [0.0, 0.0, 1.0]));
        (right, cross(right, forward))
    }

    /// Model millimetres per screen point at the target, for a view
    /// `height` points high.
    pub fn mm_per_point(&self, height: f32) -> f32 {
        2.0 * self.distance * (FOV / 2.0).tan() / height.max(1.0)
    }

    /// Looks at the whole scene.
    pub fn fit(&mut self, scene: &Scene) {
        let (centre, radius) = scene.bounds().unwrap_or(([0.0; 3], 5000.0));
        self.target = centre;
        self.distance = 1.2 * radius.max(1000.0) / (FOV / 2.0).sin();
    }

    /// Projection times view, column by column as the shader reads it.
    fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        let eye = self.eye();
        let forward = normalize(sub(self.target, eye));
        let (right, up) = self.right_up();
        let view = [
            [right[0], right[1], right[2], -dot(right, eye)],
            [up[0], up[1], up[2], -dot(up, eye)],
            [-forward[0], -forward[1], -forward[2], dot(forward, eye)],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let (near, far) = (self.distance / 100.0, self.distance * 100.0);
        let f = 1.0 / (FOV / 2.0).tan();
        let proj = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), near * far / (near - far)],
            [0.0, 0.0, -1.0, 0.0],
        ];
        // Columns of proj × view.
        std::array::from_fn(|c| {
            std::array::from_fn(|r| (0..4).map(|k| proj[r][k] * view[k][c]).sum())
        })
    }
}

/// Prepares the GPU pipeline of the 3D view. Called once, when the window
/// opens.
pub fn install(state: &egui_wgpu::RenderState) {
    let device = &state.device;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scene"),
        source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scene"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene uniforms"),
        size: UNIFORM_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniforms.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, 1 => Float32x3, 2 => Float32x3
                ],
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format: state.target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    state.renderer.write().callback_resources.insert(Resources {
        pipeline,
        uniforms,
        bind_group,
        vertices: None,
    });
}

const UNIFORM_SIZE: u64 = 20 * 4;

struct Resources {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// The generation of the scene in GPU memory, its buffer and its
    /// vertex count.
    vertices: Option<(u64, wgpu::Buffer, u32)>,
}

/// One frame of the 3D view: the scene seen through the camera. Turned into
/// an egui shape with [`egui_wgpu::Callback::new_paint_callback`].
pub struct Frame {
    /// The triangles.
    pub scene: Arc<Scene>,
    /// The camera.
    pub camera: Camera,
    /// Width over height of the view.
    pub aspect: f32,
}

impl Frame {
    /// The egui shape that draws this frame in `rect`.
    pub fn shape(self, rect: egui::Rect) -> egui::Shape {
        egui::Shape::Callback(egui_wgpu::Callback::new_paint_callback(rect, self))
    }
}

impl CallbackTrait for Frame {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(r) = resources.get_mut::<Resources>() else {
            return Vec::new();
        };
        let sun = normalize([0.4, -0.7, 0.6]);
        let uniforms: Vec<f32> = self
            .camera
            .view_proj(self.aspect)
            .iter()
            .flatten()
            .copied()
            .chain([sun[0], sun[1], sun[2], 0.0])
            .collect();
        queue.write_buffer(&r.uniforms, 0, &bytes(&uniforms));
        let current = r.vertices.as_ref().map(|v| v.0);
        if current != Some(self.scene.generation) {
            let data = bytes(self.scene.vertices.as_flattened());
            r.vertices = (!data.is_empty()).then(|| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("scene vertices"),
                    size: data.len() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&buffer, 0, &data);
                (
                    self.scene.generation,
                    buffer,
                    self.scene.vertices.len() as u32,
                )
            });
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(r) = resources.get::<Resources>() else {
            return;
        };
        let Some((_, buffer, count)) = &r.vertices else {
            return;
        };
        pass.set_pipeline(&r.pipeline);
        pass.set_bind_group(0, &r.bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..*count, 0..1);
    }
}

fn bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_ne_bytes()).collect()
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = length(a);
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archibim_model::Id;

    #[test]
    fn a_box_has_twelve_triangles_and_the_camera_sees_its_centre() {
        let prism = archibim_geometry::Prism {
            element: Id(1),
            role: None,
            // Clockwise on purpose: it is turned around.
            outline: vec![
                Point::new(0.0, 0.0),
                Point::new(0.0, 1000.0),
                Point::new(1000.0, 1000.0),
                Point::new(1000.0, 0.0),
            ],
            bottom: 0.0,
            top: 3000.0,
        };
        let scene = Scene::new(1, &[(&Block::from(&prism), [1.0, 1.0, 1.0])]);
        assert_eq!(scene.vertices.len(), 12 * 3);
        // Every side normal points away from the box.
        for v in &scene.vertices {
            let out = [v[0] - 500.0, v[1] - 500.0, v[2] - 1500.0];
            assert!(dot(out, [v[3], v[4], v[5]]) > 0.0, "{v:?}");
        }
        let mut camera = Camera::default();
        camera.fit(&scene);
        let m = camera.view_proj(1.5);
        let clip: [f32; 4] = std::array::from_fn(|r| {
            (0..4)
                .map(|c| m[c][r] * [500.0, 500.0, 1500.0, 1.0][c])
                .sum()
        });
        let (x, y, z) = (clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]);
        assert!(x.abs() < 1e-3 && y.abs() < 1e-3, "centred: {x} {y}");
        assert!((0.0..1.0).contains(&z), "between near and far: {z}");
    }
}
