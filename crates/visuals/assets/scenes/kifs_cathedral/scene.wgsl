// Distance (x) and orbit trap (y) of the folded box.
fn kifs(p0: vec3f, p: Params) -> vec2f {
    var q = p0;
    var s = 1.0;
    var trap = 1e9;
    for (var i = 0; i < p.iters; i++) {
        q = abs(q);
        if (q.x < q.y) { q = q.yxz; }
        if (q.x < q.z) { q = q.zyx; }
        if (q.y < q.z) { q = q.xzy; }
        q = rot_y(rot_x(q, p.fold), p.fold * 0.7);
        q = q * p.scale - p.offset * (p.scale - 1.0);
        s *= p.scale;
        trap = min(trap, length(q));
    }
    return vec2f(sd_box(q, vec3f(1.0)) / s, trap);
}

fn world(pos: vec3f, p: Params) -> vec2f {
    // Repeat along the flight axis so the cathedral never ends.
    var q = pos;
    q.z = q.z - 4.0 * round(q.z / 4.0);
    return kifs(q, p);
}

fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    let z = m.motion * p.fly;
    let ro = vec3f(0.0, 0.0, z);
    let ta = ro + vec3f(sin(m.motion * 0.05) * 0.3, cos(m.motion * 0.04) * 0.2, 1.0);
    let rd = camera_ray(ro, ta, uv, p.fov);

    var t = 0.05;
    var steps = 0;
    var trap = 1e9;
    var hit = false;
    for (; steps < p.steps; steps++) {
        let d = world(ro + rd * t, p);
        trap = min(trap, d.y);
        if (d.x < 0.001 * t) {
            hit = true;
            break;
        }
        t += d.x * 0.9;
        if (t > 12.0) {
            break;
        }
    }
    let ao = 1.0 - f32(steps) / f32(p.steps);
    var col = mix(p.color_a, p.color_b, clamp(trap * 0.3, 0.0, 1.0)) * ao * ao;
    if (!hit) {
        col *= 0.3;
    }
    col += p.glow * 0.02 * f32(steps) / f32(p.steps) * 40.0 * mix(p.color_b, vec3f(1.0), 0.3) * exp(-trap);
    col = fog(col, p.color_b * 0.05, t, p.fog);
    return vec4f(col, 0.9);
}
