#[macro_use]
extern crate glium;
extern crate lyon;


use lyon::extra::rust_logo::build_logo_path;
use lyon::path::builder::*;
use lyon::math::*;
use lyon::tessellation::geometry_builder::{VertexConstructor, VertexBuffers, BuffersBuilder};
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::{FillTessellator, FillOptions};
use lyon::tessellation::{StrokeTessellator, StrokeOptions};
use lyon::tessellation;
use lyon::path::default::Path;
use glium::{glutin, Surface};
use glium::glutin::Event;
use glium::glutin::EventsLoop;
use glium::glutin::KeyboardInput;

use std::ops::Rem;

const DEFAULT_WINDOW_WIDTH: f32 = 800.0;
const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
const PRIM_BUFFER_LEN: usize = 64;

fn main() {
    println!("== gfx-rs example ==");
    println!("Controls:");
    println!("  Arrow keys: scrolling");
    println!("  PgUp/PgDown: zoom in/out");
    println!("  w: toggle wireframe mode");
    println!("  b: toggle drawing the background");
    println!("  a/z: increase/decrease the stroke width");


    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions(DEFAULT_WINDOW_HEIGHT as u32, DEFAULT_WINDOW_WIDTH as u32)
        .with_decorations(true)
        .with_title("Simple tessellation".to_string());
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let num_instances  = 32;
    let tolerance = 0.02;

    // Build a Path for the rust logo.
    let mut builder = SvgPathBuilder::new(Path::builder());
    build_logo_path(&mut builder);
    let path = builder.build();

    let mut logo_fill_geometry: VertexBuffers<GpuVertex> = VertexBuffers::new();
    let mut logo_stroke_geometry : VertexBuffers<GpuVertex> = VertexBuffers::new();

    let stroke_prim_id = 0;
    let fill_prim_id = 1;

    FillTessellator::new()
        .tessellate_path(
            path.path_iter(),
            &FillOptions::tolerance(tolerance),
            &mut BuffersBuilder::new(&mut logo_fill_geometry, GpuVertexFillCtor),
        )
        .unwrap();

    StrokeTessellator::new().tessellate_path(
        path.path_iter(),
        &StrokeOptions::tolerance(tolerance).dont_apply_line_width(),
        &mut BuffersBuilder::new(
            &mut logo_stroke_geometry,
            GpuVertexStrokeCtor,
        ),
    );

 let mut instances = vec![
        IdVertex { a_prim_id: 0 },
        ];

 for i in 1..num_instances+1 {
 	 instances.push(IdVertex{
 		a_prim_id:i as i32,
 	});
 }

 let instance_buf = glium::VertexBuffer::new(&display, &instances).unwrap();

    let mut bg_geometry: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(point(-1.0, -1.0), size(2.0, 2.0)),
        &mut BuffersBuilder::new(&mut bg_geometry, BgVertexCtor),
    );

    let mut cpu_primitives = Vec::with_capacity(PRIM_BUFFER_LEN);
    for _ in 0..PRIM_BUFFER_LEN {
        cpu_primitives.push(Primitive {
            color: [1.0, 0.0, 0.0, 1.0],
            z_index: 0,
            width: 0.0,
            translate: [0.0, 0.0],
        });
    }

    // Stroke primitive
    cpu_primitives[stroke_prim_id] = Primitive {
        color: [0.0, 0.0, 0.0, 1.0],
        z_index: num_instances as i32 + 2,
        width: 1.0,
        translate: [0.0, 0.0],
    };
    // Main fill primitive
    cpu_primitives[fill_prim_id] = Primitive {
        color: [1.0, 1.0, 1.0, 1.0],
        z_index: num_instances as i32 + 1,
        width: 0.0,
        translate: [0.0, 0.0],
    };
    // Intance primitives
    for idx in (fill_prim_id + 1)..(fill_prim_id + num_instances) {
        cpu_primitives[idx].z_index = (num_instances - idx + 1) as i32;
        cpu_primitives[idx].color = [
            (0.1 * idx as f32).rem(1.0),
            (0.5 * idx as f32).rem(1.0),
            (0.9 * idx as f32).rem(1.0),
            1.0,
        ];
    }

    //background shader
    let bg_program = glium::Program::from_source(
        &display,
        BACKGROUND_VERTEX_SHADER,
        BACKGROUND_FRAGMENT_SHADER,
        None,
    ).unwrap();

    //rust logo shader
    let logo_program = glium::Program::from_source(&display, VERTEX_SHADER, FRAGMENT_SHADER, None)
        .unwrap();

    //background vertices and indices
    let bg_vertex_buffer = glium::VertexBuffer::new(&display, &bg_geometry.vertices).unwrap();
    let bg_index_buffer = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &bg_geometry.indices,
    ).unwrap();

    //rust logo vertices and indices
    let logo_fill_vertex_buffer = glium::VertexBuffer::new(&display, &logo_fill_geometry.vertices).unwrap();
    let logo_fill_index_buffer = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &logo_fill_geometry.indices,
    ).unwrap(); 

    let logo_stroke_vertex_buffer = glium::VertexBuffer::new(&display, &logo_stroke_geometry.vertices).unwrap();
    let logo_stroke_index_buffer = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &logo_stroke_geometry.indices,
    ).unwrap();

    let mut scene = SceneParams {
        target_zoom: 5.0,
        zoom: 0.1,
        target_scroll: vector(70.0, 70.0),
        scroll: vector(70.0, 70.0),
        show_points: false,
        show_wireframe: false,
        stroke_width: 1.0,
        target_stroke_width: 1.0,
        draw_background: true,
        cursor_position: (0.0, 0.0),
        window_size: (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
    };

    let mut frame_count: usize = 0;
    while update_inputs(&mut events_loop, &mut scene) {
        let (w, h) = display.gl_window().get_inner_size_pixels().unwrap();
        scene.window_size = (w as f32, h as f32);

        let gb = Globals {
            u_resolution: [w as f32, h as f32],
            u_zoom: scene.zoom,
            u_scroll_offset: scene.scroll.to_array(),
        };

        let ub: glium::uniforms::UniformBuffer<Globals> =
            glium::uniforms::UniformBuffer::new(&display, gb).unwrap();

        cpu_primitives[stroke_prim_id as usize].width = scene.stroke_width;
        cpu_primitives[stroke_prim_id as usize].color =
            [
                (frame_count as f32 * 0.008 - 1.6).sin() * 0.1 + 0.1,
                (frame_count as f32 * 0.005 - 1.6).sin() * 0.1 + 0.1,
                (frame_count as f32 * 0.01 - 1.6).sin() * 0.1 + 0.1,
                1.0,
            ];

        for idx in 2..(num_instances + 1) {
            cpu_primitives[idx].translate =
                [
                    (frame_count as f32 * 0.001 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                    (frame_count as f32 * 0.002 * idx as f32).sin() * (100.0 + idx as f32 * 10.0),
                ];
        }

        let mut prims = Primitives { primitives: [cpu_primitives[0]; PRIM_BUFFER_LEN] };

        prims.primitives.copy_from_slice(&cpu_primitives[..]);

        // let ub: glium::uniforms::UniformBuffer<Globals> =
        //     glium::uniforms::UniformBuffer::new(&display, gb).unwrap();

        let prim_buffer: glium::uniforms::UniformBuffer<Primitives> =
            glium::uniforms::UniformBuffer::new(&display, prims).unwrap();


        let bg_uniforms = uniform! { Globals: &ub };
        let mut target = display.draw();
        //target.clear_color(0.8, 0.8, 0.8, 1.0);
        target
            .draw(
                &bg_vertex_buffer,
                &bg_index_buffer,
                &bg_program,
                &bg_uniforms,
                &Default::default(),
            )
            .unwrap();

        let logo_uniforms = uniform!{Globals: &ub, u_primitives : &prim_buffer};
        target
            .draw(
                (&logo_fill_vertex_buffer,
                instance_buf.per_instance().unwrap()),
                &logo_fill_index_buffer,
                &logo_program,
                &logo_uniforms,
                &Default::default(),
            )
            .unwrap();

        let logo_uniforms = uniform!{Globals: &ub, u_primitives : &prim_buffer};
        target
            .draw(
               (&logo_stroke_vertex_buffer,
                instance_buf.per_instance().unwrap()),
                &logo_stroke_index_buffer,
                &logo_program,
                &logo_uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
        frame_count += 1;
    }
}

fn update_inputs(events_loop: &mut EventsLoop, scene: &mut SceneParams) -> bool {
    //use glutin::Event;
    use glutin::VirtualKeyCode;
    use glutin::WindowEvent;

    let mut status = true;

    events_loop.poll_events(|event| {
        match event {
            Event::WindowEvent { event: WindowEvent::Closed, .. } => {
                status = false;
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput {
                    state: glutin::ElementState::Pressed,
                    button: glutin::MouseButton::Left,
                    ..
                },
                ..
            } => {
                let half_width = scene.window_size.0 * 0.5;
                let half_height = scene.window_size.1 * 0.5;
                println!("X: {}, Y: {}",
                    (scene.cursor_position.0 - half_width) / scene.zoom + scene.scroll.x,
                    (scene.cursor_position.1 - half_height) / scene.zoom + scene.scroll.y,
                );
            }
            Event::WindowEvent {
                event: WindowEvent::MouseMoved { position: (x, y), .. }, ..
            } => {
                scene.cursor_position = (x as f32, y as f32);
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        state: _,
                        virtual_keycode: Some(key),
                        ..
                    },
                    ..
                },
                ..
            } => {
                match key {
                    VirtualKeyCode::Escape => {
                        status = false;
                    }
                    VirtualKeyCode::PageDown => {
                        scene.target_zoom *= 0.8;
                    }
                    VirtualKeyCode::PageUp => {
                        scene.target_zoom *= 1.25;
                    }
                    VirtualKeyCode::Left => {
                        scene.target_scroll.x -= 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Right => {
                        scene.target_scroll.x += 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Up => {
                        scene.target_scroll.y -= 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::Down => {
                        scene.target_scroll.y += 50.0 / scene.target_zoom;
                    }
                    VirtualKeyCode::P => {
                        scene.show_points = !scene.show_points;
                    }
                    VirtualKeyCode::W => {
                        scene.show_wireframe = !scene.show_wireframe;
                    }
                    VirtualKeyCode::B => {
                        scene.draw_background = !scene.draw_background;
                    }
                    VirtualKeyCode::A => {
                        scene.target_stroke_width /= 0.8;
                    }
                    VirtualKeyCode::Z => {
                        scene.target_stroke_width *= 0.8;
                    }
                    _key => {}
                }
            }
            _evt => {
                //println!("{:?}", _evt);
            }
        }
        //println!(" -- zoom: {}, scroll: {:?}", scene.target_zoom, scene.target_scroll);
    });

    scene.zoom += (scene.target_zoom - scene.zoom) / 3.0;
    scene.scroll = scene.scroll + (scene.target_scroll - scene.scroll) / 3.0;
    scene.stroke_width = scene.stroke_width +
        (scene.target_stroke_width - scene.stroke_width) / 5.0;

    return status;
}

#[derive(Clone, Copy)]
struct Primitive {
    color: [f32; 4],
    z_index: i32,
    width: f32,
    translate: [f32; 2],
}

implement_uniform_block!(Primitive, color, z_index, width, translate);

#[derive(Clone, Copy)]
struct Primitives {
    primitives: [Primitive; PRIM_BUFFER_LEN],
}

implement_uniform_block!(Primitives, primitives);

#[derive(Clone, Copy)]
struct Globals {
    u_resolution: [f32; 2],
    u_scroll_offset: [f32; 2],
    u_zoom: f32,
}


implement_uniform_block!(Globals, u_resolution, u_scroll_offset, u_zoom);

//implement_buffer_content!(Globals);

#[derive(Copy, Clone)]
struct BgVertex {
    position: [f32; 2],
}

implement_vertex!(BgVertex, position);

struct BgVertexCtor;
impl VertexConstructor<tessellation::FillVertex, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> BgVertex {
        BgVertex { position: vertex.position.to_array() }
    }
}


#[derive(Copy, Clone)]
struct GpuVertex {
    a_position: [f32; 2],
    a_normal: [f32; 2],
}

implement_vertex!(GpuVertex, a_position, a_normal);

#[derive(Copy, Clone)]
struct IdVertex{
    a_prim_id: i32, // An id pointing to the PrimData struct above.
}

implement_vertex!(IdVertex, a_prim_id);


struct GpuVertexFillCtor;
impl VertexConstructor<tessellation::FillVertex, GpuVertex> for GpuVertexFillCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        GpuVertex {
            a_position: vertex.position.to_array(),
            a_normal: vertex.normal.to_array(),
        }
    }
}

struct GpuVertexStrokeCtor;
impl VertexConstructor<tessellation::StrokeVertex, GpuVertex> for GpuVertexStrokeCtor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        debug_assert!(!vertex.advancement.is_nan());
        GpuVertex {
            a_position: vertex.position.to_array(),
            a_normal: vertex.normal.to_array(),
        }
    }
}


struct SceneParams {
    target_zoom: f32,
    zoom: f32,
    target_scroll: Vector,
    scroll: Vector,
    show_points: bool,
    show_wireframe: bool,
    stroke_width: f32,
    target_stroke_width: f32,
    draw_background: bool,
    cursor_position: (f32, f32),
    window_size: (f32, f32),
}

static BACKGROUND_VERTEX_SHADER: &'static str = &"
    #version 140
    in vec2 position;
    out vec2 v_position;

    void main() {
        gl_Position = vec4(position, 1.0, 1.0);
        v_position = position;
    }
";

// The background.
// This shader is silly and slow, but it looks nice ;)
static BACKGROUND_FRAGMENT_SHADER: &'static str = &"
    #version 140
    uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };
    in vec2 v_position;
    out vec4 out_color;

    void main() {
        vec2 px_position = v_position * vec2(1.0, -1.0) * u_resolution * 0.5;

        // #005fa4
        float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));
        out_color = mix(
            vec4(0.0, 0.47, 0.9, 1.0),
            vec4(0.0, 0.1, 0.64, 1.0),
            vignette
        );

        // TODO: properly adapt the grid while zooming in and out.
        float grid_scale = 5.0;
        if (u_zoom < 2.5) {
            grid_scale = 1.0;
        }

        vec2 pos = px_position + u_scroll_offset * u_zoom;

        if (mod(pos.x, 20.0 / grid_scale * u_zoom) <= 1.0 ||
            mod(pos.y, 20.0 / grid_scale * u_zoom) <= 1.0) {
            out_color *= 1.2;
        }

        if (mod(pos.x, 100.0 / grid_scale * u_zoom) <= 2.0 ||
            mod(pos.y, 100.0 / grid_scale * u_zoom) <= 2.0) {
            out_color *= 1.2;
        }
    }
";

pub static VERTEX_SHADER: &'static str = &"
    #version 140

    #define PRIM_BUFFER_LEN 64

    uniform Globals {
        vec2 u_resolution;
        vec2 u_scroll_offset;
        float u_zoom;
    };

    struct Primitive {
        vec4 color;
        int z_index;
        float width;
        vec2 translate;
    };
    uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec2 local_pos = a_position + a_normal * prim.width;
        vec2 world_pos = local_pos - u_scroll_offset + prim.translate;
        vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, -0.5) * u_resolution);

        float z = float(prim.z_index) / 4096.0;
        gl_Position = vec4(transformed_pos, 1.0 - z, 1.0);
        v_color = prim.color;
    }
";

pub static FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";
