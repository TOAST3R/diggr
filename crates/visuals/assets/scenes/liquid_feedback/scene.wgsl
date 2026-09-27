fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    // Flow field: the previous frame drifts along noise, zooms in and turns.
    let q = uv * p.scale;
    let t = m.motion * 0.1;
    let flow = vec2f(fbm(q + vec2f(t, 0.0), 3), fbm(q + vec2f(5.2, -t), 3)) - 0.5;
    let src = rot2(p.spin) * uv * (1.0 - p.zoom_in) + flow * p.flow;
    var col = prev(src).rgb * p.decay;

    // Emitters: a ring expanding with each beat, a spectrum line across the middle.
    let tint = mix(p.hue_a, p.hue_b, 0.5 + 0.5 * sin(m.motion * 0.25));
    let ring_r = m.beat * 1.4;
    col += tint * p.ring * (1.0 - m.beat) * exp(-abs(length(uv) - ring_r) * 60.0);
    let x = clamp((uv.x / m.aspect) * 0.5 + 0.5, 0.0, 0.999);
    let y = (band(m, i32(x * 19.0)) - 0.2) * 0.6 * sin(uv.x * 3.0 + m.motion);
    col += mix(p.hue_b, p.hue_a, x) * p.wave * exp(-abs(uv.y - y) * 90.0);
    return vec4f(col, 1.0);
}
