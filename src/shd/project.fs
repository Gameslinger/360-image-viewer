#version 330 core
#define M_2xSQRT2 2.8284271247461900976033774484194
//precision highp float;
in vec3 Ray;
uniform float scalar;
uniform sampler2D sample_projection;

void main()
{
	vec3 R = normalize(Ray);
	vec2 iRay = R.xy / (M_2xSQRT2 * sqrt(R.z + 1.0));
	vec2 iRay_scaled = scalar * iRay;

	if (length(iRay_scaled) >= 0.5 && scalar > 1.0)
		gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
	else
	{
		/* Scale from NDC to UV space */
		vec2 uv = iRay_scaled;
    uv -= 0.5;
    uv.y = -uv.y;
    uv.x = 1.-uv.x;
    gl_FragColor = vec4(texture2D(sample_projection, uv).rgb, 1.0);
	}
}
