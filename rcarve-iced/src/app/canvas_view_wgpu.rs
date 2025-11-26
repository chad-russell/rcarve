use bytemuck::{Pod, Zeroable};
use iced::mouse;
use iced::widget::shader;
use iced::widget::shader::wgpu;
use iced::widget::shader::wgpu::util::DeviceExt;
use iced::{Color, Rectangle};
use std::borrow::Cow;

use super::CameraState;
use super::canvas_view::{CanvasScene, Bounds};

pub struct WorkspaceView3D {
    pub scene: Option<CanvasScene>,
    pub camera: CameraState,
}

impl shader::Program<super::Message> for WorkspaceView3D {
    type State = ();
    type Primitive = ToolpathPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        match &self.scene {
            Some(scene) => ToolpathPrimitive::new(scene, &self.camera, bounds),
            None => ToolpathPrimitive::empty(),
        }
    }
}

#[derive(Debug)]
pub struct ToolpathPrimitive {
    vertices: Vec<Vertex>,
    camera: CameraState,
    bounds: Rectangle,
    scene_bounds: Option<Bounds>,
}

impl ToolpathPrimitive {
    pub fn new(scene: &CanvasScene, camera: &CameraState, bounds: Rectangle) -> Self {
        let mut vertices = Vec::new();

        // Helper to push lines
        let mut push_line = |p1: iced::Point, p2: iced::Point, color: Color| {
            let r = color.r;
            let g = color.g;
            let b = color.b;
            let a = color.a;

            vertices.push(Vertex {
                position: [p1.x, p1.y],
                color: [r, g, b, a],
            });
            vertices.push(Vertex {
                position: [p2.x, p2.y],
                color: [r, g, b, a],
            });
        };

        // Process Stock
        if let Some(stock) = &scene.stock {
            let color = Color::from_rgb8(200, 200, 200); // Light gray border
            let rect = stock.rect;
            // CNC coords: bottom-left is (x,y). Top-left is (x, y+h).
            // But here we just draw the rectangle lines in world space.
            let p1 = iced::Point::new(rect.x, rect.y);
            let p2 = iced::Point::new(rect.x + rect.width, rect.y);
            let p3 = iced::Point::new(rect.x + rect.width, rect.y + rect.height);
            let p4 = iced::Point::new(rect.x, rect.y + rect.height);

            push_line(p1, p2, color);
            push_line(p2, p3, color);
            push_line(p3, p4, color);
            push_line(p4, p1, color);
        }

        // Process Imports
        for import in &scene.imports {
            let color = if import.selected {
                Color::from_rgb8(0xFD, 0x7E, 0x14)
            } else {
                Color::from_rgb8(0x55, 0x55, 0x55)
            };

            for polyline in &import.polylines {
                if polyline.len() < 2 {
                    continue;
                }
                for i in 0..polyline.len() - 1 {
                    push_line(polyline[i], polyline[i+1], color);
                }
            }
        }

        // Process Toolpaths
        for toolpath in &scene.toolpaths {
            let color = if toolpath.meta.highlighted {
                toolpath.meta.color
            } else {
                Color {
                    a: 0.7,
                    ..toolpath.meta.color
                }
            };

            for segment in &toolpath.segments {
                if segment.len() < 2 {
                    continue;
                }
                for i in 0..segment.len() - 1 {
                    push_line(segment[i], segment[i+1], color);
                }
            }
        }

        // Process Debug Polygons
        for polygon in &scene.debug_polygons {
            let color = Color {
                a: 0.5,
                ..polygon.color
            };
            for segment in &polygon.segments {
                 if segment.len() < 2 {
                    continue;
                }
                for i in 0..segment.len() - 1 {
                    push_line(segment[i], segment[i+1], color);
                }
            }
        }

        // Process V-Carve Debug Visualization
        if let Some(vcarve_debug) = &scene.vcarve_debug {
            // Pre-prune Voronoi edges - light gray
            let pre_prune_color = Color::from_rgba(0.67, 0.67, 0.67, 0.5);
            for edge in &vcarve_debug.pre_prune_edges {
                push_line(edge[0], edge[1], pre_prune_color);
            }

            // Post-prune (kept) Voronoi edges - green
            let post_prune_color = Color::from_rgba(0.196, 0.804, 0.196, 0.8);
            for edge in &vcarve_debug.post_prune_edges {
                push_line(edge[0], edge[1], post_prune_color);
            }

            // Pruned (removed) edges - red
            let pruned_color = Color::from_rgba(1.0, 0.267, 0.267, 0.67);
            for edge in &vcarve_debug.pruned_edges {
                push_line(edge[0], edge[1], pruned_color);
            }

            // Crease paths - blue (separate from regular toolpath)
            let crease_color = Color::from_rgba(0.357, 0.553, 1.0, 0.93);
            for edge in &vcarve_debug.crease_paths {
                push_line(edge[0], edge[1], crease_color);
            }

            // Pocket boundary paths - cyan
            let pocket_color = Color::from_rgba(0.0, 0.808, 0.82, 0.93);
            for path in &vcarve_debug.pocket_boundary_paths {
                if path.len() < 2 {
                    continue;
                }
                for i in 0..path.len() - 1 {
                    push_line(path[i], path[i + 1], pocket_color);
                }
            }
        }

        Self {
            vertices,
            camera: camera.clone(),
            bounds,
            scene_bounds: Some(scene.bounds.clone()),
        }
    }

    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            camera: CameraState::default(),
            bounds: Rectangle::default(),
            scene_bounds: None,
        }
    }
}

impl shader::Primitive for ToolpathPrimitive {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        storage: &mut shader::Storage,
        _bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        if !storage.has::<Pipeline>() {
            storage.store(Pipeline::new(device, format));
        }

        let pipeline = storage.get_mut::<Pipeline>().unwrap();

        // Calculate projection matrix
        // We want to map World Coordinates to Window NDC.
        let matrix = if let Some(scene_bounds) = &self.scene_bounds {
            let width = self.bounds.width;
            let height = self.bounds.height;
            
            let scene_width = scene_bounds.width();
            let scene_height = scene_bounds.height();
            
            let base_scale = (width / scene_width).min(height / scene_height) * 0.9;
            let scale = base_scale * self.camera.zoom;
            
            let base_offset_x = (width - scene_width * base_scale) / 2.0;
            let base_offset_y = (height - scene_height * base_scale) / 2.0;
            
            let offset_x = base_offset_x + self.camera.pan_x;
            let offset_y = base_offset_y + self.camera.pan_y;
            
            // 1. World Space -> Widget Logical Space (relative to 0,0)
            let tx = -scene_bounds.min.x * scale + offset_x;
            let ty = scene_bounds.max.y * scale + offset_y;
            
            let world_to_widget = glam::Mat4::from_cols(
                glam::Vec4::new(scale, 0.0, 0.0, 0.0),
                glam::Vec4::new(0.0, -scale, 0.0, 0.0),
                glam::Vec4::new(0.0, 0.0, 1.0, 0.0),
                glam::Vec4::new(tx, ty, 0.0, 1.0),
            );

            // 2. Widget Logical Space -> Window Logical Space
            let widget_to_window = glam::Mat4::from_translation(glam::Vec3::new(self.bounds.x, self.bounds.y, 0.0));

            // 3. Window Logical Space -> NDC
            // viewport.logical_size() is what we need. Iced's shader::Viewport exposes physical_size() and scale_factor().
            let physical_size = viewport.physical_size();
            let scale_factor = viewport.scale_factor() as f32;
            let logical_width = physical_size.width as f32 / scale_factor;
            let logical_height = physical_size.height as f32 / scale_factor;

            let window_to_ndc = glam::Mat4::from_cols(
                glam::Vec4::new(2.0 / logical_width, 0.0, 0.0, 0.0),
                glam::Vec4::new(0.0, -2.0 / logical_height, 0.0, 0.0),
                glam::Vec4::new(0.0, 0.0, 1.0, 0.0),
                glam::Vec4::new(-1.0, 1.0, 0.0, 1.0),
            );

            window_to_ndc * widget_to_window * world_to_widget
        } else {
            glam::Mat4::IDENTITY
        };

        let uniforms = Uniforms {
            view_proj: matrix.to_cols_array(),
        };

        // Update uniforms
        pipeline.update_uniforms(device, queue, &uniforms);

        // Update vertices
        pipeline.update_geometry(device, queue, &self.vertices);
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if self.vertices.is_empty() {
            return;
        }
        
        let pipeline = storage.get::<Pipeline>().unwrap();
        pipeline.render(encoder, target, *clip_bounds, self.vertices.len() as u32);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [f32; 16],
}

struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Pipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Toolpath Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SOURCE)),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
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
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Initial dummy buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024, 
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            vertex_capacity: 0, // Set to 0 to force resize on first use
            uniform_buffer,
            bind_group,
        }
    }

    fn update_uniforms(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn update_geometry(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex]) {
        if self.vertex_capacity < vertices.len() {
            // Resize buffer
            self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.vertex_capacity = vertices.len();
        } else {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: Rectangle<u32>,
        vertex_count: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..vertex_count, 0..1);
    }
}

const SHADER_SOURCE: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Transform position
    out.position = uniforms.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
