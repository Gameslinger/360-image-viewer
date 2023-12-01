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
use std::time::{Duration, Instant};
use std::{convert::TryInto, mem::size_of, path::Path};
use std::{f32::consts::PI, fs::File};

use crate::{
    gl_safe::{Buffer, ShaderProgram, VertexArray},
    video_loader::VideoStream,
};

mod gl_safe;
mod video_loader;

type Vertex = [f32; 5];

struct RotImage {
    viewrays: [Vertex; 4],
    fov: f32,
    camera_rot: [f32; 2],
    source_fov: f32,
    twin_view: bool,
    zoom: f32,
}

struct CameraController {
    rot_mutation: [f32; 2],
    fov_mutation: f32,
    camera_rot_amount: f32,
    fov_scale_amount: f32,
    zoom_mutation: f32,
    zoom_scale_amount: f32,
    pause: bool,
}
impl Default for CameraController {
    fn default() -> Self {
        Self {
            rot_mutation: [0.0, 0.0],
            fov_mutation: 0.0,
            camera_rot_amount: 0.03,
            fov_scale_amount: 0.05,
            zoom_mutation: 0.0,
            zoom_scale_amount: 0.0003,
            pause: false,
        }
    }
}

impl CameraController {
    fn handle_inputs(&mut self, sdl: &Sdl, image: &mut RotImage) -> (bool, bool) {
        let rot_amount = self.camera_rot_amount;
        let fov_scale_amount = self.fov_scale_amount;
        let zoom_scale_amount = self.zoom_scale_amount;
        let mut update_camera = false;
        let mut exit = false;
        while let Some((event, _timestamp)) = sdl.poll_events() {
            match event {
                Event::Quit => {
                    exit = true;
                    break;
                }
                Event::Key {
                    pressed,
                    repeat: 0,
                    keycode,
                    ..
                } => match keycode {
                    SDLK_LEFT | SDLK_a => {
                        self.rot_mutation[0] += if pressed { rot_amount } else { -rot_amount };
                    }
                    SDLK_RIGHT | SDLK_d => {
                        self.rot_mutation[0] -= if pressed { rot_amount } else { -rot_amount };
                    }
                    SDLK_UP | SDLK_w => {
                        self.rot_mutation[1] += if pressed { rot_amount } else { -rot_amount };
                    }
                    SDLK_DOWN | SDLK_s => {
                        self.rot_mutation[1] -= if pressed { rot_amount } else { -rot_amount };
                    }
                    SDLK_e => {
                        self.fov_mutation -= if pressed {
                            fov_scale_amount
                        } else {
                            -fov_scale_amount
                        };
                    }
                    SDLK_q => {
                        self.fov_mutation += if pressed {
                            fov_scale_amount
                        } else {
                            -fov_scale_amount
                        };
                    }
                    SDLK_LEFTBRACKET | SDLK_r => {
                        self.zoom_mutation -= if pressed {
                            zoom_scale_amount
                        } else {
                            -zoom_scale_amount
                        };
                    }
                    SDLK_RIGHTBRACKET | SDLK_f => {
                        self.zoom_mutation += if pressed {
                            zoom_scale_amount
                        } else {
                            -zoom_scale_amount
                        };
                    }
                    SDLK_ESCAPE => {
                        exit = true;
                        break;
                    }
                    SDLK_SPACE => {
                        if pressed {
                            self.pause = !self.pause;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if self.rot_mutation[0] != 0.0
            || self.rot_mutation[1] != 0.0
            || self.fov_mutation != 0.0
            || self.zoom_mutation != 0.0
        {
            update_camera = true;
            image.fov += self.fov_mutation;
            image.zoom += self.zoom_mutation;
            image.rotate_viewrays(self.rot_mutation[0], self.rot_mutation[1]);
        }
        (update_camera, exit)
    }
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
            source_fov: 2. * PI,
            twin_view: false,
            zoom: 1.0,
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
    let is_video = true;
    let mut video_stream = VideoStream::new(filename, 300).expect("Unable to open video file");

    let mut twin_view = false;
    let mut source_fov = PI;
    if args[2] == "t" || args[2] == "twin" {
        twin_view = true;
    } else {
        source_fov = args[2]
            .parse::<f32>()
            .expect("Invalid input for source field of view!")
            * PI
            / 180.;
    }
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
        twin_view,
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
    let mut frame = video_stream
        .get_next_frame()
        .expect("Unable to get first frame of video!");
    unsafe {
        texture = gl_safe::Texture::new();
        texture
            //.load_file(&Path::new(filename))
            .load_bytes(frame.width(), frame.height(), frame.data(0))
            .expect("Could not open image and load texture!");
        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, texture.0);
        glGenerateMipmap(GL_TEXTURE_2D);
        glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_BLEND);
    }
    let scalar_location = get_shader_variable("scalar", shader_program.0);
    let zoom_location = get_shader_variable("zoom", shader_program.0);
    let twin_view_location = get_shader_variable("twin_view", shader_program.0);
    let mut controller: CameraController = Default::default();

    unsafe {
        glUniform1i(twin_view_location, if image.twin_view { 1 } else { 0 });
    }

    let mut update_frame = false;
    let mut start = Instant::now();
    //TODO: Extract framerate from source video
    let DESIRED_FPS: f32 = 29.9;
    let frame_time = Duration::from_secs_f32(1.0 / DESIRED_FPS);
    loop {
        let (update_camera, exit) = controller.handle_inputs(&sdl, &mut image);
        if exit {
            break;
        }
        unsafe {
            glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
            let end = Instant::now();
            let duration = end - start;
            if duration > frame_time && !controller.pause {
                texture
                    .load_bytes(frame.width(), frame.height(), frame.data(0))
                    .expect("Could not open image and load texture!");
                update_frame = true;
                start = Instant::now();
                frame = video_stream.get_next_frame().unwrap_or(frame);
            }
            if update_camera || update_frame {
                vbo.bind(gl_safe::BufferType::Array);
                gl_safe::buffer_data(
                    gl_safe::BufferType::Array,
                    bytemuck::cast_slice(&image.viewrays),
                    GL_DYNAMIC_DRAW,
                );
                Buffer::clear_binding(gl_safe::BufferType::Array);
                update_frame = false;
            }
            glUniform1f(scalar_location, image.get_scalar());
            glUniform1f(zoom_location, image.zoom);
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
