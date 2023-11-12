const WINDOW_TITLE: &str = "";

use beryllium::{
    events::Event,
    init::InitFlags,
    video::{CreateWinArgs, GlProfile, GlSwapInterval},
    Sdl,
};

use core::ffi::c_void;
use fermium::keycode::*;
use gl33::{gl_enumerations::*, global_loader::*};
use nalgebra_glm::{rotate_x_vec3, rotate_y_vec3, vec3};
use std::env;
use std::ffi::CString;
use std::io::Read;
use std::{convert::TryInto, mem::size_of, path::Path};
use std::{f32::consts::PI, fs::File};

use crate::gl_safe::{Buffer, ShaderProgram, VertexArray};

mod gl_safe;

type Vertex = [f32; 5];

struct RotImage {
    viewrays: [Vertex; 4],
    fov: f32,
    camera_rot: [f32; 2],
    camera_rot_amount: f32,
    source_fov: f32,
    fov_scale_amount: f32,
}

impl Default for RotImage {
    fn default() -> Self {
        Self {
            viewrays: [
                [-1., 1., 0.0, 0.0, 0.0],
                [1., 1., 0.0, 0.0, 0.0],
                [1., -1., 0.0, 0.0, 0.0],
                [-1., -1., 0.0, 0.0, 0.0],
            ],
            fov: PI / 2.0,
            camera_rot: [0., 0.],
            camera_rot_amount: 0.1,
            source_fov: 2. * PI,
            fov_scale_amount: 0.1,
        }
    }
}

impl RotImage {
    fn rotate_viewrays(&mut self, angle_x: f32, angle_y: f32) {
        let distance = self.get_distance();
        self.camera_rot[0] += angle_x;
        self.camera_rot[1] += angle_y;
        for v in &mut self.viewrays {
            let mut vec = vec3(v[0] * 0.5, v[1] * 0.5, distance);
            self.camera_rot[1] = f32::max(-PI / 2., self.camera_rot[1]);
            self.camera_rot[1] = f32::min(PI / 2., self.camera_rot[1]);
            vec = rotate_x_vec3(&vec, self.camera_rot[1]);
            vec = rotate_y_vec3(&vec, self.camera_rot[0] + PI / 2.);
            v[2] = vec[0];
            v[3] = vec[1];
            v[4] = vec[2];
        }
    }
    fn get_scalar(&self) -> f32 {
        1.0 / (self.source_fov / 4.0).sin()
    }
    fn get_distance(&self) -> f32 {
        -0.5 / (self.fov / 2.0).tan()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = args[1].as_str();
    let source_fov = if args[2] == "360" { 2. * PI } else { PI };
    // Setup the window
    let sdl = Sdl::init(InitFlags::EVERYTHING);
    sdl.set_gl_profile(GlProfile::Core).unwrap();
    sdl.set_gl_context_major_version(3).unwrap();
    sdl.set_gl_context_minor_version(3).unwrap();

    let win = sdl
        .create_gl_window(CreateWinArgs {
            title: filename,
            resizable: true,
            width: 800,
            height: 800,
            ..Default::default()
        })
        .expect("couldn't make a window and context");

    win.set_swap_interval(GlSwapInterval::Vsync)
        .expect("Failed to set swap interval");

    unsafe {
        load_global_gl(&|f_name| win.get_proc_address(f_name));
    }

    gl_safe::clear_color(0.0, 0.0, 0.0, 1.0);

    let vao = VertexArray::new().expect("Couldn't make a VAO");
    vao.bind();
    let mut image: RotImage = RotImage {
        source_fov,
        ..Default::default()
    };
    image.rotate_viewrays(0.0, 0.0);

    let vbo = Buffer::new().expect("Couldn't make a VBO");
    vbo.bind(gl_safe::BufferType::Array);
    gl_safe::buffer_data(
        gl_safe::BufferType::Array,
        bytemuck::cast_slice(&image.viewrays),
        GL_DYNAMIC_DRAW,
    );

    unsafe {
        glVertexAttribPointer(
            0,
            2,
            GL_FLOAT,
            0,
            size_of::<Vertex>().try_into().unwrap(),
            0 as *const _,
        );
        glEnableVertexAttribArray(0);
        glVertexAttribPointer(
            1,
            3,
            GL_FLOAT,
            0,
            size_of::<Vertex>().try_into().unwrap(),
            (size_of::<f32>() * 2) as *const c_void, //[(size_of::<f32>() as c_int) * 3].as_ptr() as *const _,
        );
        glEnableVertexAttribArray(1);
    }
    // Create Program
    let frag_shader = open_file("src/shd/project.fs").expect("Unable to open fragment shader");
    let vert_shader = open_file("src/shd/project.vs").expect("Unable to open vertex shader");
    let shader_program = ShaderProgram::from_vert_frag(&vert_shader, &frag_shader).unwrap();
    shader_program.use_program();
    println!("Shader compliation status: {}", shader_program.info_log());

    let texture: gl_safe::Texture;
    unsafe {
        texture = gl_safe::Texture::new();
        texture
            .load(&Path::new(filename))
            .expect("Could not open image and load texture!");
        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, texture.0);
        glGenerateMipmap(GL_TEXTURE_2D);
        glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_BLEND);
    }
    let scalar_location = get_shader_variable("scalar", shader_program.0);
    loop {
        let (update_camera, exit) = poll_events(&sdl, &mut image);
        if exit {
            break;
        }
        unsafe {
            glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
            if update_camera {
                vbo.bind(gl_safe::BufferType::Array);
                gl_safe::buffer_data(
                    gl_safe::BufferType::Array,
                    bytemuck::cast_slice(&image.viewrays),
                    GL_DYNAMIC_DRAW,
                );
                Buffer::clear_binding(gl_safe::BufferType::Array);
            }
            glUniform1f(scalar_location, image.get_scalar());
            glDrawArrays(GL_TRIANGLE_FAN, 0, 4);
            win.swap_window();
        }
    }
}

fn open_file(file_name: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(file_name)?;
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)?;
    Ok(file_contents)
}

fn get_shader_variable(str: &str, program_id: u32) -> i32 {
    unsafe {
        let c_name = CString::new(str).expect("Convert to c-string");
        let location = glGetUniformLocation(program_id, c_name.as_ptr() as *const _);
        let error = glGetError();
        if error != GL_NO_ERROR {
            panic!("Failed to find scalar location! {:?}", error);
        }
        location
    }
}

fn poll_events(sdl: &Sdl, image: &mut RotImage) -> (bool, bool) {
    let rot_amount = image.camera_rot_amount;
    let fov_scale_amount = image.fov_scale_amount;
    let mut rot_delta: [f32; 2] = [0., 0.];
    let mut fov_delta: f32 = 0.0;
    let mut update_camera = false;
    let mut exit = false;
    while let Some((event, _timestamp)) = sdl.poll_events() {
        match event {
            Event::Quit => {
                exit = true;
                break;
            }
            Event::Key {
                pressed: true,
                keycode,
                ..
            } => match keycode {
                SDLK_LEFT => {
                    rot_delta[0] += rot_amount;
                    update_camera = true;
                }
                SDLK_RIGHT => {
                    rot_delta[0] -= rot_amount;
                    update_camera = true;
                }
                SDLK_UP => {
                    rot_delta[1] += rot_amount;
                    update_camera = true;
                }
                SDLK_DOWN => {
                    rot_delta[1] -= rot_amount;
                    update_camera = true;
                }
                SDLK_e => {
                    fov_delta -= fov_scale_amount;
                    update_camera = true;
                }
                SDLK_q => {
                    fov_delta += fov_scale_amount;
                    update_camera = true;
                }
                SDLK_ESCAPE => {
                    exit = true;
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }
    if rot_delta[0] != 0.0 || rot_delta[1] != 0.0 || fov_delta != 0.0 {
        image.fov += fov_delta;
        image.rotate_viewrays(rot_delta[0], rot_delta[1]);
    }
    (update_camera, exit)
}
