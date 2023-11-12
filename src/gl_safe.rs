use std::{convert::TryInto, ffi::c_uint, path::Path};

use gl33::{
    gl_enumerations::{GL_INFO_LOG_LENGTH, GL_LINK_STATUS, GL_TEXTURE_2D, GL_TRUE},
    global_loader::*,
    GLenum, GL_ARRAY_BUFFER, GL_COMPILE_STATUS, GL_ELEMENT_ARRAY_BUFFER, GL_FILL,
    GL_FRAGMENT_SHADER, GL_FRONT_AND_BACK, GL_LINE, GL_LINEAR, GL_LINEAR_MIPMAP_LINEAR, GL_POINT,
    GL_REPEAT, GL_RGBA, GL_TEXTURE_MAG_FILTER, GL_TEXTURE_MIN_FILTER, GL_TEXTURE_WRAP_S,
    GL_TEXTURE_WRAP_T, GL_UNSIGNED_BYTE, GL_VERTEX_SHADER,
};
use image::{EncodableLayout, ImageError};

pub fn clear_color(r: f32, g: f32, b: f32, a: f32) {
    unsafe { glClearColor(r, g, b, a) }
}

pub struct VertexArray(pub c_uint);
impl VertexArray {
    pub fn new() -> Option<Self> {
        let mut vao = 0;
        unsafe { glGenVertexArrays(1, &mut vao) };
        if vao != 0 {
            Some(Self(vao))
        } else {
            None
        }
    }

    pub fn bind(&self) {
        glBindVertexArray(self.0)
    }

    pub fn clear_binding() {
        glBindVertexArray(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    Array = GL_ARRAY_BUFFER.0 as isize,
    ElementArray = GL_ELEMENT_ARRAY_BUFFER.0 as isize,
}

pub struct Buffer(pub c_uint);
impl Buffer {
    pub fn new() -> Option<Self> {
        let mut vbo = 0;
        unsafe {
            glGenBuffers(1, &mut vbo);
        }
        if vbo != 0 {
            Some(Self(vbo))
        } else {
            None
        }
    }

    pub fn bind(&self, ty: BufferType) {
        unsafe { glBindBuffer(GLenum(ty as u32), self.0) }
    }

    pub fn clear_binding(ty: BufferType) {
        unsafe { glBindBuffer(GLenum(ty as u32), 0) }
    }
}

pub fn buffer_data(ty: BufferType, data: &[u8], usage: GLenum) {
    unsafe {
        glBufferData(
            GLenum(ty as u32),
            data.len().try_into().unwrap(),
            data.as_ptr().cast(),
            usage,
        );
    }
}

pub enum ShaderType {
    Vertex = GL_VERTEX_SHADER.0 as isize,
    Fragment = GL_FRAGMENT_SHADER.0 as isize,
}

pub struct Shader(pub c_uint);
impl Shader {
    /// Makes a new shader.
    ///
    /// Prefer the [`Shader::from_source`](Shader::from_source) method.
    ///
    /// Possibly skip the direct creation of the shader object and use
    /// [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag).
    pub fn new(ty: ShaderType) -> Option<Self> {
        let shader = glCreateShader(GLenum(ty as u32));
        if shader != 0 {
            Some(Self(shader))
        } else {
            None
        }
    }

    /// Assigns a source string to the shader.
    ///
    /// Replaces any previously assigned source.
    pub fn set_source(&self, src: &str) {
        unsafe {
            glShaderSource(
                self.0,
                1,
                &(src.as_bytes().as_ptr().cast()),
                &(src.len().try_into().unwrap()),
            );
        }
    }

    /// Compiles the shader based on the current source.
    pub fn compile(&self) {
        glCompileShader(self.0);
    }

    /// Checks if the last compile was successful or not.
    pub fn compile_success(&self) -> bool {
        let mut compiled = 0;
        unsafe { glGetShaderiv(self.0, GL_COMPILE_STATUS, &mut compiled) };
        compiled == GL_TRUE.0 as i32
    }

    /// Gets the info log for the shader.
    ///
    /// Usually you use this to get the compilation log when a compile failed.
    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { glGetShaderiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            glGetShaderInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Marks a shader for deletion.
    ///
    /// Note: This _does not_ immediately delete the shader. It only marks it for
    /// deletion. If the shader has been previously attached to a program then the
    /// shader will stay allocated until it's unattached from that program.
    pub fn delete(self) {
        glDeleteShader(self.0);
    }

    /// Takes a shader type and source string and produces either the compiled
    /// shader or an error message.
    ///
    /// Prefer [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag),
    /// it makes a complete program from the vertex and fragment sources all at
    /// once.
    pub fn from_source(ty: ShaderType, source: &str) -> Result<Self, String> {
        let id = Self::new(ty).ok_or_else(|| "Couldn't allocate new shader".to_string())?;
        id.set_source(source);
        id.compile();
        if id.compile_success() {
            Ok(id)
        } else {
            let out = id.info_log();
            id.delete();
            Err(out)
        }
    }
}

pub struct ShaderProgram(pub c_uint);
impl ShaderProgram {
    /// Allocates a new program object.
    ///
    /// Prefer [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag),
    /// it makes a complete program from the vertex and fragment sources all at
    /// once.
    pub fn new() -> Option<Self> {
        let prog = glCreateProgram();
        if prog != 0 {
            Some(Self(prog))
        } else {
            None
        }
    }

    /// Attaches a shader object to this program object.
    pub fn attach_shader(&self, shader: &Shader) {
        glAttachShader(self.0, shader.0);
    }

    /// Links the various attached, compiled shader objects into a usable program.
    pub fn link_program(&self) {
        glLinkProgram(self.0);
    }

    /// Checks if the last linking operation was successful.
    pub fn link_success(&self) -> bool {
        let mut success = 0;
        unsafe { glGetProgramiv(self.0, GL_LINK_STATUS, &mut success) };
        success == GL_TRUE.0 as i32
    }

    /// Gets the log data for this program.
    ///
    /// This is usually used to check the message when a program failed to link.
    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { glGetProgramiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            glGetProgramInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Sets the program as the program to use when drawing.
    pub fn use_program(&self) {
        glUseProgram(self.0);
    }

    /// Marks the program for deletion.
    ///
    /// Note: This _does not_ immediately delete the program. If the program is
    /// currently in use it won't be deleted until it's not the active program.
    /// When a program is finally deleted and attached shaders are unattached.
    pub fn delete(self) {
        glDeleteProgram(self.0);
    }

    /// Takes a vertex shader source string and a fragment shader source string
    /// and either gets you a working program object or gets you an error message.
    ///
    /// This is the preferred way to create a simple shader program in the common
    /// case. It's just less error prone than doing all the steps yourself.
    pub fn from_vert_frag(vert: &str, frag: &str) -> Result<Self, String> {
        let p = Self::new().ok_or_else(|| "Couldn't allocate a program".to_string())?;
        let v = Shader::from_source(ShaderType::Vertex, vert)
            .map_err(|e| format!("Vertex Compile Error: {}", e))?;
        let f = Shader::from_source(ShaderType::Fragment, frag)
            .map_err(|e| format!("Fragment Compile Error: {}", e))?;
        p.attach_shader(&v);
        p.attach_shader(&f);
        p.link_program();
        v.delete();
        f.delete();
        if p.link_success() {
            Ok(p)
        } else {
            let out = format!("Program Link Error: {}", p.info_log());
            p.delete();
            Err(out)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Point = GL_POINT.0 as isize,
    Line = GL_LINE.0 as isize,
    Fill = GL_FILL.0 as isize,
}

pub fn polygon_mode(mode: PolygonMode) {
    unsafe { glPolygonMode(GL_FRONT_AND_BACK, GLenum(mode as u32)) }
}

pub struct Texture(pub c_uint);
impl Texture {
    pub unsafe fn new() -> Self {
        let mut id: c_uint = 0;
        glGenTextures(1, &mut id);
        Self(id)
    }

    pub unsafe fn bind(&self) {
        glBindTexture(GL_TEXTURE_2D, self.0);
    }

    pub unsafe fn load(&self, path: &Path) -> Result<(), ImageError> {
        self.bind();

        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT.0 as i32);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT.0 as i32);
        glTexParameteri(
            GL_TEXTURE_2D,
            GL_TEXTURE_MIN_FILTER,
            GL_LINEAR_MIPMAP_LINEAR.0 as i32,
        );
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR.0 as i32);

        let img = image::open(path)?.into_rgba8();
        glTexImage2D(
            GL_TEXTURE_2D,
            0,
            GL_RGBA.0 as i32,
            img.width() as i32,
            img.height() as i32,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            img.as_bytes().as_ptr() as *const _,
        );
        glGenerateMipmap(GL_TEXTURE_2D);
        Ok(())
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { glDeleteTextures(1, [self.0].as_ptr()) };
    }
}
