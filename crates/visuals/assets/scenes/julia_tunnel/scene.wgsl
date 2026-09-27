fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    // Blend the plane with log-polar coordinates that scroll with the music.
    let r = length(uv);
    let lp = vec2f(log(max(r, 1e-3)) - m.motion * 0.25, atan2(uv.y, uv.x) / 3.14159265);
    var z = mix(uv, lp * 1.2, p.tunnel);
    z = rot2(m.motion * p.rot_speed) * z / p.zoom;

    var trap = 1e9;
    var i = 0;
    for (; i < p.iters; i++) {
        z = csq(z) + p.julia_c;
        trap = min(trap, abs(length(z) - p.trap));
        if (dot(z, z) > 256.0) {
            break;
        }
    }
    var t = 1.0;
    if (i < p.iters) {
        // Smooth iteration count.
        t = (f32(i) - log2(log2(dot(z, z))) + 4.0) / f32(p.iters);
    }
    var col = palette(t * 3.0 + p.palette_shift + m.bar * 0.05, p.color_a, p.color_b, vec3f(1.0), vec3f(0.0, 0.15, 0.3));
    if (i == p.iters) {
        col *= 0.08;
    }
    col += p.glow * exp(-trap * 10.0) * vec3f(1.0, 0.85, 0.6);
    col *= mix(1.0, smoothstep(0.0, 0.25, r), p.tunnel);
    return vec4f(col, 0.85);
}
