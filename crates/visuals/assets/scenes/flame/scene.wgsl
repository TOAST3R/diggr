fn transform(q: vec2f, j: u32, m: Music, p: Params) -> vec2f {
    var off = p.off1;
    if (j == 1u) { off = p.off2; }
    if (j == 2u) { off = p.off3; }
    let a = p.rot[j] + m.motion * p.spin * (f32(j) + 1.0) * 0.1;
    let r = rot2(a) * q * p.scl[j] + off;
    // Variations.
    let rr = dot(r, r);
    let v_sin = sin(r);
    let v_sph = r / (rr + 1e-4);
    let v_swirl = vec2f(r.x * sin(rr) - r.y * cos(rr), r.x * cos(rr) + r.y * sin(rr));
    let w = p.variation / max(p.variation.x + p.variation.y + p.variation.z, 1e-3);
    return v_sin * w.x + v_sph * w.y + v_swirl * w.z;
}

fn simulate(i: u32, m: Music, p: Params) {
    var st = pcg(i * 9781u + u32(m.frame) * 6271u + 1u);
    var q = vec2f(rand01(st), rand01(st + 7u)) * 2.0 - 1.0;
    var c = 0.5;
    let w = p.weights / (p.weights.x + p.weights.y + p.weights.z);
    let view = rot2(m.motion * 0.02);
    for (var k = 0; k < p.steps; k++) {
        st = pcg(st);
        let r = f32(st >> 8u) / 16777216.0;
        var j = 0u;
        if (r > w.x) { j = 1u; }
        if (r > w.x + w.y) { j = 2u; }
        q = transform(q, j, m, p);
        c = (c + f32(j) * 0.5) * 0.5;
        if (k >= 6) {
            var col = mix(p.color_a, p.color_b, clamp(c * 2.0, 0.0, 1.0));
            col = mix(col, p.color_c, clamp(c * 2.0 - 1.0, 0.0, 1.0));
            splat(view * q * p.zoom, col);
        }
    }
}

fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    let a = accum_at(uv);
    let d = a.w;
    let tint = a.rgb / max(d, 1e-3);
    let bright = 1.0 - exp(-log(1.0 + d) * p.exposure * 0.5);
    return vec4f(pow(tint * bright, vec3f(0.8)), 0.75);
}
