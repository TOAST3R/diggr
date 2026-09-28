// Tunnels seen from inside: where a screen point meets the wall of a tube whose far end sways.
// The camera sits on the axis of a tube of radius 1, looking down it; `bend` moves the
// vanishing point, and the tube curves smoothly towards it, like flying through a bent pipe.

// The tube's axis on screen at depth `z` (0 at the camera, `bend` far away).
fn tube_axis(z: f32, bend: vec2f) -> vec2f {
    return bend * z / (z + 2.0);
}

// A screen point `uv` on the tube wall: (depth along the tube, angle in turns 0..1, screen
// distance from the axis at that depth). `focal` ≈ zoom. With `bend` = 0 the depth is
// `focal / length(uv)` and the angle is the plain screen angle.
fn tube_hit(uv: vec2f, focal: f32, bend: vec2f) -> vec3f {
    var d = uv;
    var z = focal / max(length(d), 1e-4);
    for (var i = 0; i < 3; i++) {
        d = uv - tube_axis(z, bend);
        z = focal / max(length(d), 1e-4);
    }
    return vec3f(z, fract(atan2(d.y, d.x) / 6.2831853), length(d));
}

// How squarely the wall faces the camera where it is `r` from the axis on screen: 0 grazing
// (far away), towards 1 close by.
fn tube_facing(r: f32, focal: f32) -> f32 {
    return r / sqrt(r * r + focal * focal);
}
