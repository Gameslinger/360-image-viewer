#version 330 core
layout (location=0) in vec2 pos;
layout (location=1) in vec3 rayvtx;
out vec3 Ray;

void main()
{
	Ray = rayvtx;
	gl_Position = vec4(pos, 0.0, 1.0);
}
