// Cells: x = 1 alive, 0..1 dying (fading by 1 / decay states per step), 0 dead;
// y = age (steps / 64); z = hue, from where on the spectrum the cell's line was born;
// w = on ring 0 only, that angle's running spectrum level (for spotting onsets).

fn is_alive(s: vec4f) -> bool {
    return s.x > 0.99;
}

// The rule for this part of the song, as (birth mask, survive mask, decay states). With
// `section_rules` on, builds, drops and breakdowns each get their own life; the rest of the song
// runs the variant's rule.
fn section_rule(m: Music, p: Params) -> vec3<i32> {
    if (p.section_rules > 0.5) {
        switch (i32(round(m.section_kind))) {
            // Build: Brian's Brain (B2/S/3), restless sparks that never settle.
            case 1: {
                return vec3<i32>(4, 0, 2);
            }
            // Drop: Star Wars (B2/S345/4), exploding lattices.
            case 2: {
                return vec3<i32>(4, 56, 3);
            }
            // Breakdown: Day & Night (B3678/S34678), calm blobs with long fading trails.
            case 3: {
                return vec3<i32>(456, 472, 8);
            }
            default: {}
        }
    }
    return vec3<i32>(p.birth, p.survive, p.decay_states);
}

// One step further towards death: alive → first dying state → … → dead.
fn fade(s: vec4f, decay: i32) -> vec4f {
    let x = s.x - 1.0 / f32(max(decay, 1));
    if (x < 0.01) {
        return vec4f(0.0);
    }
    return vec4f(x, s.y + 1.0 / 64.0, s.z, 0.0);
}

// Steps since the last outward push (0 on a push step).
fn since_push(p: Params) -> i32 {
    let k = max(p.shift_every, 1);
    return ((tick() % k) + k) % k;
}

// Beats since the last hit: a kick, or also the beat itself with `beat_hits` while the music
// has some energy (the analysis can miss kicks the ear hears; the beat grid is dependable, but
// it keeps ticking through silence).
fn hit(m: Music, p: Params) -> f32 {
    return select(m.kick, min(m.kick, m.beat), p.beat_hits > 0.5 && m.energy > 0.15);
}

// Identifies the last hit (its time in eighths of a beat), so each hit gets its own random
// choices and every step and pixel agrees on them.
fn hit_id(m: Music, p: Params) -> f32 {
    return floor((m.beats - hit(m, p)) * 8.0 + 0.5);
}

fn rule(c: vec2<i32>, m: Music, p: Params) -> vec4f {
    // A fresh grid starts as a random soup, which the pre-roll turns into living patterns, so the
    // tunnel is full from the first frame; the music then feeds new life in from the centre.
    if (since_reset() == 0) {
        let r = hash22(vec2f(f32(c.x), f32(c.y)) + m.seed * 97.0);
        let theta = (f32(c.x) + 0.5) / f32(grid().x);
        let born = select(0.0, 1.0, r.x < p.soup);
        return vec4f(born, 0.0, r.y, select(0.0, inject_level(m, theta), c.y == 0));
    }
    let law = section_rule(m, p);
    // Every `shift_every` steps the whole tunnel moves one ring outward: this step's cell then
    // comes from the ring below. In between, the automaton evolves in place.
    let src = select(c, c - vec2<i32>(0, 1), since_push(p) == 0);
    let s = cell(src);
    var n = 0u;
    var hue = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            let q = cell(src + vec2<i32>(dx, dy));
            if (is_alive(q)) {
                n += 1u;
                hue += q.z;
            }
        }
    }
    var next = vec4f(0.0);
    if (is_alive(s)) {
        let old = s.y * 64.0 >= f32(p.max_age);
        if (((u32(law.y) >> n) & 1u) == 1u && !old) {
            next = vec4f(1.0, s.y + 1.0 / 64.0, s.z, 0.0);
        } else {
            next = fade(s, law.z);
        }
    } else if (s.x <= 0.0) {
        if (((u32(law.x) >> n) & 1u) == 1u) {
            next = vec4f(1.0, 0.0, hue / f32(max(n, 1u)), 0.0);
        }
    } else {
        next = fade(s, law.z);
    }

    // Kick interference: right after each kick a random band of rings is corrupted (cells flip
    // at random), and Life takes the noise and runs with it as the band flies outward.
    if (hit(m, p) < 0.2 && p.kick_noise > 0.0) {
        let kick_at = hit_id(m, p);
        let h = hash22(vec2f(kick_at, m.seed * 31.0));
        let rings = grid().y;
        let r0 = rings / 16 + i32(h.x * f32(rings - rings / 4));
        let w = 2 + i32(h.y * f32(rings / 10));
        if (c.y >= r0 && c.y < r0 + w) {
            let f = hash21(vec2f(f32(c.x) + kick_at * 13.0, f32(c.y) + f32(tick())));
            if (f < p.kick_noise) {
                next = select(vec4f(1.0, 0.0, fract(h.x + h.y), 0.0), vec4f(0.0), is_alive(next));
            }
        }
    }

    // Inner ring: life is born where the spectrum rises above its own recent level (an onset),
    // so quiet and loud passages both feed the tunnel, and kicks scatter seeds.
    if (c.y == 0) {
        let theta = (f32(c.x) + 0.5) / f32(grid().x);
        let level = inject_level(m, theta);
        let avg = cell(c).w;
        let seed = u32(tick()) * 2654435761u ^ u32(c.x) * 40503u ^ u32(m.seed * 16777216.0);
        let kicked = rand01(seed) < p.kick_seed * pulse(hit(m, p), 6.0);
        if (level > avg + p.sensitivity || kicked) {
            next = vec4f(1.0, 0.0, 1.0 - abs(1.0 - 2.0 * theta), 0.0);
        }
        next.w = mix(avg, level, 0.15);
    }
    return next;
}

// A cell at (theta, ring) in grid units (floored); empty outside the rings.
fn cell_at(t: f32, r: f32) -> vec4f {
    return cell(vec2<i32>(i32(floor(t)), i32(floor(r))));
}

fn cell_color(s: vec4f, p: Params) -> vec3f {
    return palette(
        s.z * 0.6 + s.y * 0.35 + p.palette_shift,
        p.color_a,
        p.color_b,
        vec3f(1.0),
        vec3f(0.0, 0.33, 0.67),
    );
}

// How brightly a cell shines: living cells glow, dying ones dim out.
fn cell_light(s: vec4f, p: Params) -> f32 {
    return select(0.35 * s.x * s.x, 1.0 + p.glow, is_alive(s));
}

// Slope of smoothstep(a, b, x) at x.
fn smoothstep_slope(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    return 6.0 * t * (1.0 - t) / (b - a);
}

// The tunnel at ring position `u` and angle `theta` (turns). Each cell is a raised tile with a
// bevelled rim lit from the vanishing point; with `rich`, living cells also bleed a soft halo into
// the gaps and every cell drags a comet tail behind it as the tunnel flies outward.
fn tunnel(u: f32, theta: f32, energy: f32, p: Params, rich: bool) -> vec3f {
    let tu = fract(theta) * f32(grid().x);
    let iu = floor(u);
    let it = floor(tu);
    let fu = u - iu;
    let ft = tu - it;
    let s = cell_at(it, iu);

    // Bevel: the tile's height rises from its edges; its slope tilts the normal.
    let a = 0.03;
    let b = a + max(p.bevel, 0.01);
    let eu = min(fu, 1.0 - fu);
    let et = min(ft, 1.0 - ft);
    let hu = smoothstep(a, b, eu);
    let ht = smoothstep(a, b, et);
    var slope = vec2f(0.0);
    if (hu < ht) {
        slope.x = select(-1.0, 1.0, fu < 0.5) * smoothstep_slope(a, b, eu);
    } else {
        slope.y = select(-1.0, 1.0, ft < 0.5) * smoothstep_slope(a, b, et);
    }
    let n = normalize(vec3f(-slope * 0.08, 1.0));
    // Light from the vanishing point (the −u side), a little off to one side.
    let l = normalize(vec3f(-0.7, 0.3, 0.65));
    let diffuse = max(dot(n, l), 0.0);
    let spec = pow(max(reflect(-l, n).z, 0.0), 24.0);
    let shade = mix(1.0, 0.35 + 0.9 * diffuse, min(p.bevel * 4.0, 1.0));
    let tile = min(hu, ht);
    var c = cell_color(s, p) * cell_light(s, p) * tile * shade;
    c += vec3f(spec) * 0.5 * tile * select(0.0, 1.0, is_alive(s)) * min(p.bevel * 4.0, 1.0);

    if (!rich) {
        return c;
    }
    // Halo: living cells glow into the gaps around them. Only the cell itself and the three
    // neighbours on this pixel's side of it are near enough to matter.
    if (p.halo > 0.0) {
        let side = vec2f(select(-1.0, 1.0, fu >= 0.5), select(-1.0, 1.0, ft >= 0.5));
        var halo = vec3f(0.0);
        for (var k = 0; k < 4; k++) {
            let o = vec2f(f32(k & 1), f32(k >> 1u)) * side;
            let q = select(cell_at(it + o.y, iu + o.x), s, k == 0);
            if (is_alive(q)) {
                let d = vec2f(fu - 0.5 - o.x, ft - 0.5 - o.y);
                halo += cell_color(q, p) * exp(-dot(d, d) * 5.0);
            }
        }
        c += halo * p.halo * 0.25;
    }

    // Tails: the cells just ahead (further out) streak back towards the centre.
    if (p.tails > 0.0) {
        let column = smoothstep(0.1, 0.35, et);
        var tail = vec3f(0.0);
        for (var k = 1; k <= 2; k++) {
            let q = cell_at(it, iu + f32(k));
            let behind = iu + f32(k) + 0.5 - u;
            tail += cell_color(q, p) * q.x * exp(-behind * 1.4 / p.tails);
        }
        c += tail * column * 0.3;
    }

    // A faint lattice every 8 rings and angles keeps the tunnel visible when little lives.
    let gu = fract(u / 8.0);
    let gt = fract(tu / 8.0);
    let near = min(min(gu, 1.0 - gu), min(gt, 1.0 - gt)) * 8.0;
    let lines = 1.0 - smoothstep(0.0, 0.08, near);
    c += vec3f(0.02, 0.03, 0.05) * (0.5 + energy) * lines * step(0.0, u);
    return c;
}

fn scene(uv_in: vec2f, m: Music, p: Params) -> vec4f {
    // Kick interference, like an analogue signal breaking up for an instant: rows of the screen
    // jump sideways, bands of rings tear, the colours split and static flickers over it.
    let g = p.glitch * pulse(hit(m, p), 6.0);
    let kick_at = hit_id(m, p);
    let row = floor(uv_in.y * 24.0);
    let jumps = step(0.55, hash21(vec2f(row, kick_at + 3.0)));
    let uv = uv_in + vec2f((hash21(vec2f(row, kick_at)) - 0.5) * 0.3 * g * jumps, 0.0);

    // With `tube` up, the tunnel is a pipe whose far end sways over the phrase (the camera flies
    // through its bends), lit by a headlight and fading into the distance.
    let sway = p.tube * p.sway * vec2f(sin(m.motion * 0.19635), 0.6 * sin(m.motion * 0.0982 + 1.3));
    let hit3 = tube_hit(uv, 1.0, sway);
    let r = max(hit3.z, 1e-4);
    let rings = f32(grid().y);
    // Log-polar depth: ring 0 at r_min (the vanishing point), the last ring at r_max.
    let depth = log(r / p.r_min) / log(p.r_max / p.r_min) * (rings - 1.0);
    // Glide: between two pushes every ring slides one ring outward, so the tunnel flows instead
    // of jumping on the beat.
    let glide = (f32(since_push(p)) + tick_phase()) / f32(max(p.shift_every, 1));
    // Kicks make the tunnel lunge forward and settle back.
    let u = depth + 1.0 - glide - p.kick_punch * pulse(hit(m, p), 8.0);
    // Angle in turns with the bass (theta 0) at the bottom, twisted with depth and spinning.
    var theta = hit3.y + 0.25 + p.twist * depth + p.spin * m.motion / 16.0;
    let band = floor(u / 6.0);
    let tears = step(0.6, hash21(vec2f(band + 7.0, kick_at)));
    theta += (hash21(vec2f(band, kick_at)) - 0.5) * 0.15 * g * tears;

    // The colour split costs three lookups, so only while the picture is breaking up.
    var c: vec3f;
    if (g > 0.01) {
        let split = 0.02 * g;
        c = vec3f(
            tunnel(u, theta + split, m.energy, p, false).r,
            tunnel(u, theta, m.energy, p, true).g,
            tunnel(u, theta - split, m.energy, p, false).b,
        );
    } else {
        c = tunnel(u, theta, m.energy, p, true);
    }
    let k = min(g, 1.0);
    c *= 1.0 - k * 0.45 * step(0.5, fract(uv_in.y * 90.0 + m.time * 37.0));
    let noise = hash21(floor(uv_in * vec2f(m.res_x, m.res_y) * 0.25) + fract(m.time * 13.0) * 97.0);
    c += vec3f(noise - 0.3) * 0.35 * k;

    // Headlight and distance fog for the tube; a flat tunnel (tube 0) is evenly lit.
    let facing = tube_facing(r, 1.0);
    c *= mix(1.0, (0.3 + 1.4 * facing) * exp(-0.02 * hit3.x), p.tube);

    // The tiny inner rings dissolve into a dark centre, and the rim fades out.
    c *= smoothstep(p.r_min, p.r_min * 3.0, r) * (1.0 - smoothstep(rings - 6.0, rings, u));
    return vec4f(max(c, vec3f(0.0)), 0.9);
}
