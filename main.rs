#[crate_id = "triangle"];
#[no_uv];

extern crate native;

extern crate gl;
extern crate hgl;
extern crate glfw = "glfw-rs";

use std::mem::size_of;
use std::rand::Rng;
use std::iter::AdditiveIterator;

use gl::types::GLint;
use hgl::{Shader, Program, Triangles, Vbo, Vao};

static VERTEX_SHADER: &'static str = "
#version 140

in vec2 position;
in vec3 color;
uniform vec3 const_color;
out vec3 Color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    if (all(equal(const_color, vec3(0.0)))) {
        Color = color;
    } else {
        Color = const_color;
    }
}";

static FRAGMENT_SHADER: &'static str = "
#version 140
out vec4 out_color;
in vec3 Color;

void main() {
    out_color = vec4(Color, 1.0);
}";

#[deriving(Eq)]
enum ShapeToDraw {
    Triangle,
    SierpinskiPoints,
    RandomLines
}

impl ShapeToDraw {
    fn to_prim(&self) -> hgl::Primitive {
        match *self {
            Triangle => hgl::Triangles,
            SierpinskiPoints => hgl::Points,
            RandomLines => hgl::Lines
        }
    }
}

static TRIANGLE_DATA: &'static [f32] = &[0.0, 0.5, 1.0, 0.0, 0.0,
                                         0.5,-0.5, 0.0, 1.0, 0.0,
                                        -0.5,-0.5, 0.0, 0.0, 1.0];

#[start]
fn main(argc: int, argv: **u8) -> int {
    native::start(argc, argv, proc() {
        glfw::set_error_callback(box glfw::LogErrorHandler);
        glfw::start(proc() {
            glfw::window_hint::context_version(3, 1);
            // glfw::window_hint::opengl_profile(glfw::OpenGlCoreProfile);
            let window = glfw::Window::create(800, 600, "Lab 1", glfw::Windowed).unwrap();
            window.set_mouse_button_polling(true);
            window.make_context_current();
            gl::load_with(glfw::get_proc_address);

            gl::Viewport(0, 0, 800, 600);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);

            // this could be a *lot* more efficient if it made smarter use of
            // VAOs

            let vao = Vao::new();
            vao.bind();
            let program = Program::link([Shader::compile(VERTEX_SHADER, hgl::VertexShader).unwrap(),
                                         Shader::compile(FRAGMENT_SHADER, hgl::FragmentShader).unwrap()]).unwrap();
            program.bind_frag(0, "out_color");
            program.bind();

            let mut rng = std::rand::task_rng();
            let mut to_draw = Triangle;


            let tri_vbo = Vbo::from_data(TRIANGLE_DATA, hgl::buffer::StaticDraw);
            let mut sierp_vbo;
            let mut line_vbo;

            let mut previous = RandomLines;
            let mut num_indices: GLint = 3; // default for triangle

            while !window.should_close() {
                glfw::poll_events();

                for (_, event) in window.flush_events() {
                    match event {
                        glfw::MouseButtonEvent(glfw::MouseButtonLeft, glfw::Release, _) => {
                            // Cycle the thing forward
                            to_draw = match to_draw {
                                Triangle => SierpinskiPoints,
                                SierpinskiPoints => RandomLines,
                                RandomLines => Triangle
                            }
                        },
                        _ => {}
                    }
                }

                gl::Clear(gl::COLOR_BUFFER_BIT);

                if previous != to_draw {
                    match to_draw {
                        Triangle => {
                            tri_vbo.bind();
                            vao.enable_attrib(&program, "position", gl::FLOAT, 2, 5*size_of::<f32>() as i32, 0);
                            vao.enable_attrib(&program, "color", gl::FLOAT, 3, 5*size_of::<f32>() as i32, 2*size_of::<f32>());
                            num_indices = 3;
                        },
                        SierpinskiPoints => {
                            gl::Uniform3f(program.uniform("const_color"), 0.0, 1.0, 0.0);
                            let r = rng.gen_range(1500u, 30000);
                            let points = sierpinski([(0.0, 0.5), (0.5, -0.5), (-0.5, -0.5)], r, rng);
                            sierp_vbo = Vbo::from_data(points, hgl::buffer::StreamDraw);
                            sierp_vbo.bind();
                            vao.enable_attrib(&program, "position", gl::FLOAT, 2, 0, 0);
                            num_indices = points.len() as GLint;
                        },
                        RandomLines => {
                            let cgen: || -> f32 = || rng.gen_range(-1.0f32, 1.0);

                            gl::Uniform3f(program.uniform("const_color"), 0.0, 0.0, 0.0);
                            let points = std::vec::from_fn(3, |_| (cgen(), cgen(), cgen(), cgen(), cgen()));
                            line_vbo = Vbo::from_data::<(f32, f32, f32, f32, f32)>(points, hgl::buffer::StreamDraw);
                            line_vbo.bind();
                            vao.enable_attrib(&program, "position", gl::FLOAT, 2, 5*size_of::<f32>() as i32, 0);
                            vao.enable_attrib(&program, "color", gl::FLOAT, 3, 5*size_of::<f32>() as i32, 2*size_of::<f32>());
                            num_indices = points.len() as GLint;
                            drop(cgen);
                        }
                    }
                    previous = to_draw;
                }
                vao.draw_array(previous.to_prim(), 0, num_indices);
                window.swap_buffers();
            }
        });
    });
    0
}

/// Create an approximation of the Sierpinski Triangle, as points.
fn sierpinski<R: Rng>(vertices: [(f32, f32), ..3], iterations: uint, mut rng: R) -> ~[(f32, f32)] {
    fn avg((a1, b1): (f32, f32), (a2, b2): (f32, f32)) -> (f32, f32) {
        (((a1 + a2) / 2.0), ((b1 + b2) / 2.0))
    }

    let mut p  = avg(rng.choose(vertices), {
        let mut x = (rng.gen_range::<f32>(-1.0, 1.0), rng.gen_range::<f32>(-1.0, 1.0));
        while !in_triangle(vertices, x) {
            // if at first you do not succeed, try, and try again
            x = (rng.gen_range::<f32>(-1.0, 1.0), rng.gen_range::<f32>(-1.0, 1.0));
        }
        x
    });
    let mut points = ~[p];
    for _ in range(0, iterations) {
        p = avg(rng.choose(vertices), p);
        points.push(p);
    }
    points
}

fn in_triangle(vertices: [(f32, f32), ..3], point: (f32, f32)) -> bool {
    // jeez...
    let midpoint = (vertices.iter().map(|t| t.val0()).sum() / 3.0, vertices.iter().map(|t| t.val1()).sum() / 3.0);
    let ab: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[0], vertices[1]);
        (x, ((b.val1() - a.val1()) / (b.val0() - a.val0()) * (x - b.val0())) - b.val1())
    };
    let ac: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[0], vertices[2]);
        (x, ((b.val1() - a.val1()) / (b.val0() - a.val0()) * (x - b.val0())) - b.val1())
    };
    let bc: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[1], vertices[2]);
        (x, ((b.val1() - a.val1()) / (b.val0() - a.val0()) * (x - b.val0())) - b.val1())
    };

    let dirab = midpoint < ab(midpoint.val0());
    let dirac = midpoint < ac(midpoint.val0());
    let dirbc = midpoint < bc(midpoint.val0());

    if     ((point < ab(point.val0())) == dirab)
        && ((point < ac(point.val0())) == dirac)
        && ((point < bc(point.val0())) == dirbc)
    {
        true
    } else {
        false
    }
}

#[test]
fn in_triangle_smoke_test() {
    let tri = [(0.0, 0.5), (0.5, -0.5), (-0.5, -0.5)];
    assert!(in_triangle(tri, (0, 0)));
}
