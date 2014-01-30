#[no_uv];
extern mod gl;
extern mod hgl;
extern mod glfw;
extern mod extra;
extern mod native;

use std::mem::size_of;
use std::rand::Rng;
use std::iter::AdditiveIterator;

use hgl::{Shader, Program, Triangles, Vbo, Vao};
use gl::types::GLint;
use glfw::{Window, MouseButton, Action, Modifiers};

#[link(name="glfw")]
extern {}

static VERTEX_SHADER: &'static str = "
#version 330

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
#version 330
out vec4 out_color;
in vec3 Color;

void main() {
    out_color = vec4(Color, 1.0);
}";

static mut TO_DRAW: ShapeToDraw = Triangle;

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

struct MouseButtonCallback;

impl glfw::MouseButtonCallback for MouseButtonCallback {
    fn call(&self, _window: &Window, button: MouseButton, action: Action, _modifiers: Modifiers) {
        if button == glfw::MouseButtonLeft && action == glfw::Release {
            unsafe {
                TO_DRAW = match TO_DRAW {
                    Triangle => SierpinskiPoints,
                    SierpinskiPoints => RandomLines,
                    RandomLines => Triangle
                }
            }
        }
    }
}

#[start]
fn main(argc: int, argv: **u8) -> int {
    native::start(argc, argv, proc() {
        glfw::set_error_callback(~glfw::LogErrorHandler);
        glfw::start(proc() {
            glfw::window_hint::context_version(3, 3);
            glfw::window_hint::opengl_profile(glfw::OpenGlCoreProfile);
            let window = glfw::Window::create(800, 600, "HGL", glfw::Windowed).unwrap();
            window.set_mouse_button_callback(~MouseButtonCallback);
            window.make_context_current();
            gl::load_with(glfw::get_proc_address);

            gl::Viewport(0, 0, 800, 600);

            // this could be a *lot* more efficient if it made smarter use of
            // VAOs

            let vao = Vao::new();
            vao.activate();
            let program = Program::link([Shader::compile(VERTEX_SHADER, hgl::VertexShader).unwrap(),
                                         Shader::compile(FRAGMENT_SHADER, hgl::FragmentShader).unwrap()]).unwrap();
            program.bind_frag(0, "out_color");
            program.activate();

            let mut rng = std::rand::task_rng();

            let tri_vbo = Vbo::from_data(TRIANGLE_DATA, hgl::StaticDraw).unwrap();
            let mut sierp_vbo;
            let mut line_vbo;

            let mut previous = RandomLines;
            let mut num_indices: GLint = 3; // default for triangle

            let cgen: || -> f32 = || rng.gen_range(-1.0f32, 1.0);

            while !window.should_close() {
                glfw::poll_events();
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                if previous != unsafe { TO_DRAW } {
                    match unsafe { TO_DRAW } {
                        Triangle => {
                            tri_vbo.activate();
                            vao.enable_attrib(&program, "position", 2, 5*size_of::<f32>() as i32, 0);
                            vao.enable_attrib(&program, "color", 3, 5*size_of::<f32>() as i32, 2*size_of::<f32>());
                            num_indices = 3;
                        },
                        SierpinskiPoints => {
                            gl::Uniform3f(program.uniform("const_color"), 0.0, 1.0, 0.0);
                            let points = sierpinski([(0.0, 0.5), (0.5, -0.5), (-0.5, -0.5)],
                                                    rng.gen_range(1500u, 30000), rng);
                            sierp_vbo = Vbo::from_data(points, hgl::StreamDraw).unwrap();
                            sierp_vbo.activate();
                            vao.enable_attrib(&program, "position", 2, 0, 0);
                            num_indices = points.len() as GLint;
                        },
                        RandomLines => {
                            gl::Uniform3f(program.uniform("const_color"), 0.0, 0.0, 0.0);
                            let points = std::vec::from_fn(rng.gen_range(36u, 300),
                                |_| (cgen(), cgen(), cgen(), cgen(), cgen()));
                            line_vbo = Vbo::from_data::<(f32, f32, f32, f32, f32)>(points, hgl::StreamDraw).unwrap();
                            line_vbo.activate();
                            vao.enable_attrib(&program, "position", 2, 5*size_of::<f32>() as i32, 0);
                            vao.enable_attrib(&program, "color", 3, 5*size_of::<f32>() as i32, 2*size_of::<f32>());
                            num_indices = points.len() as GLint;
                        }
                    }
                    previous = unsafe { TO_DRAW }
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
    let midpoint = (vertices.iter().map(|t| t.n0()).sum() / 3.0, vertices.iter().map(|t| t.n1()).sum() / 3.0);
    let ab: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[0], vertices[1]);
        (x, ((b.n1() - a.n1()) / (b.n0() - a.n0()) * (x - b.n0())) - b.n1())
    };
    let ac: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[0], vertices[2]);
        (x, ((b.n1() - a.n1()) / (b.n0() - a.n0()) * (x - b.n0())) - b.n1())
    };
    let bc: |f32| -> (f32, f32) = |x| {
        let (a, b) = (vertices[1], vertices[2]);
        (x, ((b.n1() - a.n1()) / (b.n0() - a.n0()) * (x - b.n0())) - b.n1())
    };

    let dirab = midpoint < ab(midpoint.n0());
    let dirac = midpoint < ac(midpoint.n0());
    let dirbc = midpoint < bc(midpoint.n0());

    if     ((point < ab(point.n0())) == dirab)
        && ((point < ac(point.n0())) == dirac)
        && ((point < bc(point.n0())) == dirbc)
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
