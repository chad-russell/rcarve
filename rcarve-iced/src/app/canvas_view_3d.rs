use bytemuck::{Pod, Zeroable};
use iced::mouse;
use iced::widget::shader;
use iced::widget::shader::wgpu;
use iced::widget::shader::wgpu::util::DeviceExt;
use iced::{Color, Rectangle, event};
use std::borrow::Cow;

use super::Camera3DState;
use super::Message;
use rcarve::StockSpec;

/// 3D scene data for rendering
#[derive(Debug, Clone)]
pub struct Scene3D {
    pub stock: Option<Stock3D>,
    pub toolpaths: Vec<Toolpath3D>,
    pub curves: Vec<Curve3D>,
}

#[derive(Debug, Clone)]
pub struct Stock3D {
    pub origin: (f32, f32, f32),
    pub width: f32,
    pub height: f32,
    pub thickness: f32,
}

impl Stock3D {
    pub fn from_stock_spec(spec: &StockSpec) -> Self {
        let origin = spec.origin.unwrap_or((0.0, 0.0, 0.0));
        Self {
            origin: (origin.0 as f32, origin.1 as f32, origin.2 as f32),
            width: spec.width as f32,
            height: spec.height as f32,
            thickness: spec.thickness as f32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toolpath3D {
    pub segments: Vec<Vec<(f32, f32, f32)>>,
    pub color: Color,
    pub highlighted: bool,
}

#[derive(Debug, Clone)]
pub struct Curve3D {
    pub segments: Vec<Vec<(f32, f32, f32)>>,
    pub color: Color,
    pub selected: bool,
}

pub struct Workspace3DView {
    pub scene: Option<Scene3D>,
    pub camera: Camera3DState,
    pub wireframe_mode: bool,
}

impl shader::Program<Message> for Workspace3DView {
    type State = ();
    type Primitive = Scene3DPrimitive;

    fn update(
        &self,
        _state: &mut Self::State,
        event: shader::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _shell: &mut iced::advanced::Shell<'_, Message>,
    ) -> (event::Status, Option<Message>) {
        match event {
            shader::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.position_in(bounds).is_some() {
                    let scroll_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 100.0,
                    };
                    (
                        event::Status::Captured,
                        Some(Message::Canvas3DZoom(scroll_delta)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            shader::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    (
                        event::Status::Captured,
                        Some(Message::Canvas3DOrbitStart(pos)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            shader::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    (
                        event::Status::Captured,
                        Some(Message::Canvas3DPanStart(pos)),
                    )
                } else {
                    (event::Status::Ignored, None)
                }
            }
            shader::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.camera.orbit_start.is_some() {
                    (event::Status::Captured, Some(Message::Canvas3DOrbitEnd))
                } else {
                    (event::Status::Ignored, None)
                }
            }
            shader::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                if self.camera.pan_start.is_some() {
                    (event::Status::Captured, Some(Message::Canvas3DPanEnd))
                } else {
                    (event::Status::Ignored, None)
                }
            }
            shader::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if self.camera.orbit_start.is_some() {
                        (
                            event::Status::Captured,
                            Some(Message::Canvas3DOrbitUpdate(pos)),
                        )
                    } else if self.camera.pan_start.is_some() {
                        (
                            event::Status::Captured,
                            Some(Message::Canvas3DPanUpdate(pos)),
                        )
                    } else {
                        (event::Status::Ignored, None)
                    }
                } else {
                    (event::Status::Ignored, None)
                }
            }
            _ => (event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        Scene3DPrimitive::new(&self.scene, &self.camera, bounds, self.wireframe_mode)
    }
}

#[derive(Debug)]
pub struct Scene3DPrimitive {
    line_vertices: Vec<Vertex3D>,
    triangle_vertices: Vec<Vertex3D>,
    camera: Camera3DState,
    #[allow(dead_code)]
    bounds: Rectangle,
    #[allow(dead_code)]
    wireframe_mode: bool,
}

impl Scene3DPrimitive {
    pub fn new(
        scene: &Option<Scene3D>,
        camera: &Camera3DState,
        bounds: Rectangle,
        wireframe_mode: bool,
    ) -> Self {
        let mut line_vertices = Vec::new();
        let mut triangle_vertices = Vec::new();

        if let Some(scene) = scene {
            // Generate stock geometry
            if let Some(stock) = &scene.stock {
                if wireframe_mode {
                    Self::generate_stock_wireframe(stock, &mut line_vertices);
                } else {
                    Self::generate_stock_solid(stock, &mut triangle_vertices);
                    // Also draw wireframe on top for clarity
                    Self::generate_stock_wireframe(stock, &mut line_vertices);
                }
            }

            // Generate toolpath geometry
            for toolpath in &scene.toolpaths {
                Self::generate_toolpath_lines(toolpath, &mut line_vertices);
            }

            // Generate curve geometry
            for curve in &scene.curves {
                Self::generate_curve_lines(curve, &mut line_vertices);
            }
        }

        Self {
            line_vertices,
            triangle_vertices,
            camera: camera.clone(),
            bounds,
            wireframe_mode,
        }
    }

    fn generate_stock_wireframe(stock: &Stock3D, vertices: &mut Vec<Vertex3D>) {
        let (ox, oy, oz) = stock.origin;
        let w = stock.width;
        let h = stock.height;
        let t = stock.thickness;

        // Stock vertices (8 corners)
        // Bottom face (Z = oz - t, since Z-negative is down)
        let bottom_z = oz - t;
        let top_z = oz;

        let corners = [
            // Bottom face
            [ox, oy, bottom_z],           // 0: bottom-front-left
            [ox + w, oy, bottom_z],       // 1: bottom-front-right
            [ox + w, oy + h, bottom_z],   // 2: bottom-back-right
            [ox, oy + h, bottom_z],       // 3: bottom-back-left
            // Top face
            [ox, oy, top_z],              // 4: top-front-left
            [ox + w, oy, top_z],          // 5: top-front-right
            [ox + w, oy + h, top_z],      // 6: top-back-right
            [ox, oy + h, top_z],          // 7: top-back-left
        ];

        let edge_color = [0.7, 0.7, 0.7, 1.0]; // Light gray

        // 12 edges
        let edges = [
            // Bottom face
            (0, 1), (1, 2), (2, 3), (3, 0),
            // Top face
            (4, 5), (5, 6), (6, 7), (7, 4),
            // Vertical edges
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];

        for (i, j) in edges {
            vertices.push(Vertex3D {
                position: corners[i],
                color: edge_color,
            });
            vertices.push(Vertex3D {
                position: corners[j],
                color: edge_color,
            });
        }
    }

    fn generate_stock_solid(stock: &Stock3D, vertices: &mut Vec<Vertex3D>) {
        let (ox, oy, oz) = stock.origin;
        let w = stock.width;
        let h = stock.height;
        let t = stock.thickness;

        let bottom_z = oz - t;
        let top_z = oz;

        let corners = [
            // Bottom face
            [ox, oy, bottom_z],           // 0
            [ox + w, oy, bottom_z],       // 1
            [ox + w, oy + h, bottom_z],   // 2
            [ox, oy + h, bottom_z],       // 3
            // Top face
            [ox, oy, top_z],              // 4
            [ox + w, oy, top_z],          // 5
            [ox + w, oy + h, top_z],      // 6
            [ox, oy + h, top_z],          // 7
        ];

        let face_color = [0.6, 0.55, 0.45, 0.3]; // Semi-transparent wood-ish color

        // 6 faces, 2 triangles each = 12 triangles
        let faces = [
            // Bottom face (normal -Z)
            [0, 2, 1], [0, 3, 2],
            // Top face (normal +Z)
            [4, 5, 6], [4, 6, 7],
            // Front face (normal -Y)
            [0, 1, 5], [0, 5, 4],
            // Back face (normal +Y)
            [2, 3, 7], [2, 7, 6],
            // Left face (normal -X)
            [0, 4, 7], [0, 7, 3],
            // Right face (normal +X)
            [1, 2, 6], [1, 6, 5],
        ];

        for tri in faces {
            for &idx in &tri {
                vertices.push(Vertex3D {
                    position: corners[idx],
                    color: face_color,
                });
            }
        }
    }

    fn generate_toolpath_lines(toolpath: &Toolpath3D, vertices: &mut Vec<Vertex3D>) {
        let color = if toolpath.highlighted {
            [toolpath.color.r, toolpath.color.g, toolpath.color.b, 1.0]
        } else {
            [toolpath.color.r, toolpath.color.g, toolpath.color.b, 0.8]
        };

        for segment in &toolpath.segments {
            if segment.len() < 2 {
                continue;
            }
            for i in 0..segment.len() - 1 {
                let (x1, y1, z1) = segment[i];
                let (x2, y2, z2) = segment[i + 1];
                vertices.push(Vertex3D {
                    position: [x1, y1, z1],
                    color,
                });
                vertices.push(Vertex3D {
                    position: [x2, y2, z2],
                    color,
                });
            }
        }
    }

    fn generate_curve_lines(curve: &Curve3D, vertices: &mut Vec<Vertex3D>) {
        let color = if curve.selected {
            [0.99, 0.49, 0.08, 1.0] // Orange for selected
        } else {
            [curve.color.r, curve.color.g, curve.color.b, 1.0]
        };

        for segment in &curve.segments {
            if segment.len() < 2 {
                continue;
            }
            for i in 0..segment.len() - 1 {
                let (x1, y1, z1) = segment[i];
                let (x2, y2, z2) = segment[i + 1];
                vertices.push(Vertex3D {
                    position: [x1, y1, z1],
                    color,
                });
                vertices.push(Vertex3D {
                    position: [x2, y2, z2],
                    color,
                });
            }
        }
    }
}

impl shader::Primitive for Scene3DPrimitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut shader::Storage,
        _bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        if !storage.has::<Pipeline3D>() {
            storage.store(Pipeline3D::new(device, format));
        }

        let pipeline = storage.get_mut::<Pipeline3D>().unwrap();

        // Ensure depth texture matches viewport size
        let physical_size = viewport.physical_size();
        pipeline.ensure_depth_texture(device, physical_size.width, physical_size.height);

        // Calculate view-projection matrix
        let aspect_ratio = physical_size.width as f32 / physical_size.height as f32;
        let view_proj = self.camera.view_projection_matrix(aspect_ratio);

        let uniforms = Uniforms3D {
            view_proj: view_proj.to_cols_array(),
        };

        pipeline.update_uniforms(queue, &uniforms);
        pipeline.update_line_geometry(device, queue, &self.line_vertices);
        pipeline.update_triangle_geometry(device, queue, &self.triangle_vertices);
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let pipeline = storage.get::<Pipeline3D>().unwrap();
        pipeline.render(
            encoder,
            target,
            *clip_bounds,
            self.line_vertices.len() as u32,
            self.triangle_vertices.len() as u32,
        );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex3D {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex3D {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3D>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms3D {
    view_proj: [f32; 16],
}

struct Pipeline3D {
    line_pipeline: wgpu::RenderPipeline,
    triangle_pipeline: wgpu::RenderPipeline,
    line_vertex_buffer: wgpu::Buffer,
    line_vertex_capacity: usize,
    triangle_vertex_buffer: wgpu::Buffer,
    triangle_vertex_capacity: usize,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_texture: Option<DepthTexture>,
}

struct DepthTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl Pipeline3D {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SOURCE_3D)),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D Uniform Buffer"),
            size: std::mem::size_of::<Uniforms3D>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D Bind Group Layout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let depth_stencil = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        // Line pipeline
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex3D::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: depth_stencil.clone(),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Triangle pipeline (for solid stock)
        let triangle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Triangle Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex3D::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None, // Disable culling to see through
                ..Default::default()
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Initial dummy buffers
        let line_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D Line Vertex Buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let triangle_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D Triangle Vertex Buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            line_pipeline,
            triangle_pipeline,
            line_vertex_buffer,
            line_vertex_capacity: 0,
            triangle_vertex_buffer,
            triangle_vertex_capacity: 0,
            uniform_buffer,
            bind_group,
            depth_texture: None,
        }
    }

    fn update_uniforms(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms3D) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn update_line_geometry(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex3D]) {
        if vertices.is_empty() {
            return;
        }
        if self.line_vertex_capacity < vertices.len() {
            self.line_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D Line Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.line_vertex_capacity = vertices.len();
        } else {
            queue.write_buffer(&self.line_vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    fn update_triangle_geometry(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex3D]) {
        if vertices.is_empty() {
            return;
        }
        if self.triangle_vertex_capacity < vertices.len() {
            self.triangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3D Triangle Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.triangle_vertex_capacity = vertices.len();
        } else {
            queue.write_buffer(&self.triangle_vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    fn ensure_depth_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let needs_new = match &self.depth_texture {
            None => true,
            Some(dt) => dt.size != (width, height),
        };

        if needs_new && width > 0 && height > 0 {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("3D Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_texture = Some(DepthTexture {
                texture,
                view,
                size: (width, height),
            });
        }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: Rectangle<u32>,
        line_vertex_count: u32,
        triangle_vertex_count: u32,
    ) {
        // We need the depth texture for proper 3D rendering
        // For now, skip if no depth texture
        let depth_view = match &self.depth_texture {
            Some(dt) => &dt.view,
            None => return,
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3D Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
        pass.set_bind_group(0, &self.bind_group, &[]);

        // Draw triangles first (solid stock)
        if triangle_vertex_count > 0 {
            pass.set_pipeline(&self.triangle_pipeline);
            pass.set_vertex_buffer(0, self.triangle_vertex_buffer.slice(..));
            pass.draw(0..triangle_vertex_count, 0..1);
        }

        // Draw lines on top (wireframe and toolpaths)
        if line_vertex_count > 0 {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            pass.draw(0..line_vertex_count, 0..1);
        }
    }
}

const SHADER_SOURCE_3D: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

