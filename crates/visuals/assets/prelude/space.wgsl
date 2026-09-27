// Rotations, signed distance functions and camera helpers for raymarching.

fn rot2(a: f32) -> mat2x2f {
    let c = cos(a);
    let s = sin(a);
    return mat2x2f(c, s, -s, c);
}

fn rot_x(p: vec3f, a: f32) -> vec3f {
    let r = rot2(a) * p.yz;
    return vec3f(p.x, r);
}

fn rot_y(p: vec3f, a: f32) -> vec3f {
    let r = rot2(a) * p.xz;
    return vec3f(r.x, p.y, r.y);
}

fn rot_z(p: vec3f, a: f32) -> vec3f {
    return vec3f(rot2(a) * p.xy, p.z);
}

fn sd_sphere(p: vec3f, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_box(p: vec3f, b: vec3f) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sd_torus(p: vec3f, t: vec2f) -> f32 {
    return length(vec2f(length(p.xz) - t.x, p.y)) - t.y;
}

// Polynomial smooth minimum.
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// Ray direction for a camera at `ro` looking at `ta`; `zoom` ≈ focal length.
fn camera_ray(ro: vec3f, ta: vec3f, uv: vec2f, zoom: f32) -> vec3f {
    let f = normalize(ta - ro);
    let r = normalize(cross(vec3f(0.0, 1.0, 0.0), f));
    let u = cross(f, r);
    return normalize(uv.x * r + uv.y * u + zoom * f);
}

// Cheap fog / glow falloff for marched distances.
fn fog(col: vec3f, fog_col: vec3f, dist: f32, density: f32) -> vec3f {
    return mix(fog_col, col, exp(-density * dist * dist));
}
