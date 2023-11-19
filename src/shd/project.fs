#version 330 core
#define M_2xSQRT2 2.8284271247461900976033774484194
//precision highp float;
in vec3 Ray;
uniform float scalar;
uniform sampler2D sample_projection;
//uniform bool twin_view; 
uniform float zoom; //= 0.9280;
uniform bool twin_view;

void main()
{
	vec3 R = normalize(Ray);
  vec2 iRay;
  if(twin_view)
    iRay = R.xy / (M_2xSQRT2 * sqrt(abs(R.z) + 1.0));
  else
    iRay = R.xy / (M_2xSQRT2 * sqrt(R.z + 1.0));
  vec2 iRay_scaled = scalar * iRay;
  vec2 uv = iRay_scaled;
  if (twin_view)
    uv *= zoom;
  uv -= 0.5;
  uv.y = -uv.y;
  uv.x = 1.-uv.x;
  if (twin_view) {
    uv.x /= 2;
  }
  if(!twin_view && length(iRay_scaled) >= 0.5 && scalar > 1.0)
    gl_FragColor = vec4(0.0,0.0,0.0,1.0);
  else if(twin_view && R.z < 0) {
    uv.x = 0.5 - uv.x;
    uv.x += 0.5;
    gl_FragColor = vec4(texture2D(sample_projection, uv).rgb, 1.0);
  } else {
    gl_FragColor = vec4(texture2D(sample_projection, uv).rgb, 1.0);
	}
}
