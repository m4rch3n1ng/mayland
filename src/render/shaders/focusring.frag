precision mediump float;

uniform float alpha;
uniform vec2 size;
varying vec2 v_coords;

uniform vec3 color;
uniform float thickness;

void main() {
	vec2 location = v_coords * size;

	float b_alpha = 0.0;
	if (location.y <= thickness
		|| location.x <= thickness
		|| size.x - location.x <= thickness
		|| size.y - location.y <= thickness) {
		b_alpha = 1.0;
	}

	vec4 mix_color = vec4(color, alpha) * b_alpha;
	gl_FragColor = mix_color;
}
