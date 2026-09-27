// Palettes and color helpers.

// Cosine palette (Inigo Quilez): a + b * cos(2π (c t + d)).
fn palette(t: f32, a: vec3f, b: vec3f, c: vec3f, d: vec3f) -> vec3f {
    return a + b * cos(6.2831853 * (c * t + d));
}

fn hsv2rgb(c: vec3f) -> vec3f {
    let k = vec3f(1.0, 2.0 / 3.0, 1.0 / 3.0);
    let p = abs(fract(c.xxx + k) * 6.0 - 3.0);
    return c.z * mix(vec3f(1.0), clamp(p - 1.0, vec3f(0.0), vec3f(1.0)), c.y);
}

// Rotates a color's hue by `turns` (0..1) around the grey axis.
fn hue_shift(col: vec3f, turns: f32) -> vec3f {
    let k = vec3f(0.57735);
    let a = turns * 6.2831853;
    let c = cos(a);
    return col * c + cross(k, col) * sin(a) + k * dot(k, col) * (1.0 - c);
}
