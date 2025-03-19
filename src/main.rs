mod framework;

use bytemuck::{Pod, Zeroable};
use simple_wgpu::{
    BindGroup, BindGroupBuilder, Buffer, ColorAttachment, ColorTargetState, CommandEncoder,
    Context, DrawCall, RasteriserState, RenderPipeline, RenderPipelineBuilder, RenderTexture,
    Shader, VertexBufferLayout,
};
use std::mem;
use std::f32::consts;
use wgpu::include_wgsl;

// ----- Vertex Data Structure -----

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],  // Using color instead of texture coordinates
}

// ----- Cube Implementation -----

struct Cube {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: usize,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    wireframe_pipeline: Option<RenderPipeline>,
}

impl Cube {
    fn new(config: &wgpu::SurfaceConfiguration, context: &Context) -> Self {
        // Create vertex and index data
        let (vertices, indices) = Self::create_cube_geometry();
        
        // Create buffers
        let vertex_buffer = Buffer::with_data(
            Some("Cube Vertices"),
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(&vertices),
            context,
        );

        let index_buffer = Buffer::with_data(
            Some("Cube Indices"),
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&indices),
            context,
        );

        // Create transformation matrix
        let aspect_ratio = config.width as f32 / config.height as f32;
        let transform_matrix = Self::create_view_projection_matrix(aspect_ratio);
        let transform_ref: &[f32; 16] = transform_matrix.as_ref();
        
        // Create uniform buffer
        let uniform_buffer = Buffer::with_data(
            Some("Transform Matrix"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(transform_ref),
            context,
        );

        // Create bind group - much simpler now without texture
        let bind_group = BindGroupBuilder::new()
            .buffer(0, wgpu::ShaderStages::VERTEX, &uniform_buffer.uniform_binding(), None)
            .build();

        // Create shader
        let shader = Shader::new(include_wgsl!("shader.wgsl"), context);

        // Create pipeline
        let vertex_layout = Self::create_vertex_layout();
        let render_pipeline = RenderPipelineBuilder::with_vertex(
            &shader.entry_point("vs_main"),
            vertex_layout.clone(),
        )
        .fragment(&shader.entry_point("fs_main"), [Some(Default::default())])
        .build();

        // Create wireframe pipeline if supported
        let wireframe_pipeline = if context.device().features().contains(wgpu_types::Features::POLYGON_MODE_LINE) {
            let pipeline = RenderPipelineBuilder::with_vertex(
                &shader.entry_point("vs_main"), 
                vertex_layout,
            )
            .fragment(
                &shader.entry_point("fs_wire"),
                [Some(ColorTargetState {
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            operation: wgpu::BlendOperation::Add,
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            )
            .build();
            
            Some(pipeline)
        } else {
            None
        };

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len(),
            uniform_buffer,
            bind_group,
            render_pipeline,
            wireframe_pipeline,
        }
    }

    fn create_vertex_layout() -> [VertexBufferLayout; 1] {
        let vertex_size = mem::size_of::<Vertex>();
        
        [VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vec![
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }]
    }

    fn create_cube_geometry() -> (Vec<Vertex>, Vec<u16>) {
        // Colors for each face
        let top_color = [0.9, 0.4, 0.3, 1.0];      // Reddish
        let bottom_color = [0.3, 0.4, 0.9, 1.0];   // Bluish
        let right_color = [0.3, 0.8, 0.3, 1.0];    // Greenish
        let left_color = [0.8, 0.3, 0.8, 1.0];     // Purplish
        let front_color = [0.9, 0.9, 0.3, 1.0];    // Yellowish
        let back_color = [0.3, 0.9, 0.9, 1.0];     // Cyanish
        
        let vertices = [
            // Top face (0, 0, 1)
            Vertex { position: [-1.0, -1.0, 1.0, 1.0], color: top_color },
            Vertex { position: [1.0, -1.0, 1.0, 1.0], color: top_color },
            Vertex { position: [1.0, 1.0, 1.0, 1.0], color: top_color },
            Vertex { position: [-1.0, 1.0, 1.0, 1.0], color: top_color },
            
            // Bottom face (0, 0, -1)
            Vertex { position: [-1.0, 1.0, -1.0, 1.0], color: bottom_color },
            Vertex { position: [1.0, 1.0, -1.0, 1.0], color: bottom_color },
            Vertex { position: [1.0, -1.0, -1.0, 1.0], color: bottom_color },
            Vertex { position: [-1.0, -1.0, -1.0, 1.0], color: bottom_color },
            
            // Right face (1, 0, 0)
            Vertex { position: [1.0, -1.0, -1.0, 1.0], color: right_color },
            Vertex { position: [1.0, 1.0, -1.0, 1.0], color: right_color },
            Vertex { position: [1.0, 1.0, 1.0, 1.0], color: right_color },
            Vertex { position: [1.0, -1.0, 1.0, 1.0], color: right_color },
            
            // Left face (-1, 0, 0)
            Vertex { position: [-1.0, -1.0, 1.0, 1.0], color: left_color },
            Vertex { position: [-1.0, 1.0, 1.0, 1.0], color: left_color },
            Vertex { position: [-1.0, 1.0, -1.0, 1.0], color: left_color },
            Vertex { position: [-1.0, -1.0, -1.0, 1.0], color: left_color },
            
            // Front face (0, 1, 0)
            Vertex { position: [1.0, 1.0, -1.0, 1.0], color: front_color },
            Vertex { position: [-1.0, 1.0, -1.0, 1.0], color: front_color },
            Vertex { position: [-1.0, 1.0, 1.0, 1.0], color: front_color },
            Vertex { position: [1.0, 1.0, 1.0, 1.0], color: front_color },
            
            // Back face (0, -1, 0)
            Vertex { position: [1.0, -1.0, 1.0, 1.0], color: back_color },
            Vertex { position: [-1.0, -1.0, 1.0, 1.0], color: back_color },
            Vertex { position: [-1.0, -1.0, -1.0, 1.0], color: back_color },
            Vertex { position: [1.0, -1.0, -1.0, 1.0], color: back_color },
        ];

        let indices: Vec<u16> = vec![
            0, 1, 2, 2, 3, 0,       // top
            4, 5, 6, 6, 7, 4,       // bottom
            8, 9, 10, 10, 11, 8,    // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // front
            20, 21, 22, 22, 23, 20, // back
        ];

        (vertices.to_vec(), indices)
    }

    fn create_view_projection_matrix(aspect_ratio: f32) -> glam::Mat4 {
        let projection = glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 10.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(1.5f32, -5.0, 3.0),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
        projection * view
    }

    fn update_transform_matrix(&mut self, aspect_ratio: f32, context: &Context) {
        let transform = Self::create_view_projection_matrix(aspect_ratio);
        let transform_ref: &[f32; 16] = transform.as_ref();
        self.uniform_buffer.write(bytemuck::cast_slice(transform_ref), context);
    }
}

// ----- Framework Implementation -----

impl framework::Main for Cube {
    fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        context: &Context,
    ) -> Self {
        Cube::new(config, context)
    }

    fn update(&mut self, _event: winit::event::WindowEvent) {
        // Empty - No dynamic updates in this simple example
    }

    fn resize(&mut self, config: &wgpu::SurfaceConfiguration, context: &Context) {
        let aspect_ratio = config.width as f32 / config.height as f32;
        self.update_transform_matrix(aspect_ratio, context);
    }

    fn render(&mut self, target: &RenderTexture, context: &Context) {
        context.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = CommandEncoder::new(None, context);
        
        // Begin render pass
        let mut render_pass = encoder.render_pass(
            None,
            vec![ColorAttachment {
                target: target.clone(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu_types::StoreOp::Store,
                },
            }],
            None,
            Some(Default::default()),
        );

        // Draw solid cube
        render_pass.draw(DrawCall {
            bind_groups: vec![self.bind_group.clone()],
            bind_group_offsets: vec![vec![]],
            pipeline: self.render_pipeline.clone(),
            vertices: vec![self.vertex_buffer.slice(..)],
            indices: Some(self.index_buffer.slice(..)),
            element_range: 0..self.index_count,
            instance_range: 0..1,
            rasteriser_state: RasteriserState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
        });

        // Draw wireframe if supported
        if let Some(ref pipeline) = self.wireframe_pipeline {
            render_pass.draw(DrawCall {
                bind_groups: vec![self.bind_group.clone()],
                bind_group_offsets: vec![vec![]],
                pipeline: pipeline.clone(),
                vertices: vec![self.vertex_buffer.slice(..)],
                indices: Some(self.index_buffer.slice(..)),
                element_range: 0..self.index_count,
                instance_range: 0..1,
                rasteriser_state: RasteriserState {
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Line,
                    ..Default::default()
                },
            });
        }
    }
}

fn main() {
    framework::run::<Cube>("Simple Colored Cube");
}
