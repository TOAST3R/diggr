// Gray-Scott reaction-diffusion on the polar grid: two chemicals, A (the substrate) and B (the
// growth), spread and react into spots, stripes and coral. Cells: x = 1 − A (the depletion: A
// rests near 1, where half floats are too coarse for the small feed steps, so the complement
// near 0 is stored instead), y = B, z = the hue the growth carries (from where on the spectrum it
// was seeded), w = on ring 0 only, that angle's running spectrum level (for spotting onsets).

// A cell for the chemistry. Inside the inner ring lies fresh substrate (all zero, as `cell`
// gives), which the flow carries in; the outer edge is closed (it mirrors the last ring), so it
// neither feeds nor drains the growth.
fn chem(c: vec2<i32>) -> vec4f {
    return cell(vec2<i32>(c.x, min(c.y, grid().y - 1)));
}

// The four side neighbours weigh 0.2, the corners 0.05: the classic 3×3 Laplacian.
fn laplacian(c: vec2<i32>, centre: vec4f) -> vec4f {
    let sides = chem(c + vec2<i32>(1, 0)) + chem(c - vec2<i32>(1, 0)) + chem(c + vec2<i32>(0, 1))
        + chem(c - vec2<i32>(0, 1));
    let corners = chem(c + vec2<i32>(1, 1)) + chem(c + vec2<i32>(-1, 1)) + chem(c + vec2<i32>(1, -1))
        + chem(c + vec2<i32>(-1, -1));
    return sides * 0.2 + corners * 0.05 - centre;
}

// Growth planted in a cell: B up, and the substrate around it partly used, as the reaction
// needs to take hold.
fn plant(s: vec2f, amount: f32) -> vec2f {
    return vec2f(max(s.x, 0.5 * amount), max(s.y, 0.5 * amount));
}

// Beats since the last hit: a kick, or the beat itself while the music has some energy.
fn hit(m: Music) -> f32 {
    return select(m.kick, min(m.kick, m.beat), m.energy > 0.15);
}

fn rule(c: vec2<i32>, m: Music, p: Params) -> vec4f {
    let theta = (f32(c.x) + 0.5) / f32(grid().x);
    // A fresh grid is all substrate with scattered seeds of growth (in 4×4 blocks, so they
    // survive), which the pre-roll grows into coral.
    if (since_reset() == 0) {
        let block = vec2f(vec2<i32>(c.x / 4, c.y / 4));
        let r = hash22(block + m.seed * 97.0);
        let b = select(0.0, 1.0, r.x < p.soup);
        return vec4f(b * 0.5, b * 0.5, r.y, select(0.0, inject_level(m, theta), c.y == 0));
    }
    // Upwind flow: every pass the tunnel drifts `flow` rings outward.
    let here = cell(c);
    let s = mix(here, chem(c - vec2<i32>(0, 1)), p.flow);
    let lap = laplacian(c, here);
    let d = s.x;
    let b = s.y;
    let abb = (1.0 - d) * b * b;
    let feed = p.feed;
    let kill = p.kill;
    // dA = ∇²A − AB² + F(1 − A), so the depletion moves the other way.
    var nd = d + (1.0 * lap.x + abb - feed * d);
    var nb = b + (0.5 * lap.y + abb - (feed + kill) * b);
    // The hue spreads with the growth.
    var hue = s.z + 0.2 * lap.z;

    // Kicks splash growth into a random band of rings, where it blooms as it flies outward.
    let g = hit(m);
    if (g < 0.1 && p.splash > 0.0) {
        let id = floor((m.beats - g) * 8.0 + 0.5);
        let h = hash22(vec2f(id, m.seed * 31.0));
        let rings = grid().y;
        let r0 = rings / 16 + i32(h.x * f32(rings / 2));
        let spot = vec2<i32>(c.x / 3, c.y / 3);
        let f = hash21(vec2f(vec2<i32>(spot.x + i32(id) * 7, spot.y)));
        if (c.y >= r0 && c.y < r0 + 3 + rings / 16 && f < p.splash) {
            let q = plant(vec2f(nd, nb), 1.0);
            nd = q.x;
            nb = q.y;
            hue = fract(h.x + h.y);
        }
    }

    // Inner rings: growth is seeded where the spectrum rises above its own recent level.
    var w = 0.0;
    if (c.y <= 2) {
        let level = inject_level(m, theta);
        let avg = cell(vec2<i32>(c.x, 0)).w;
        if (level > avg + p.sensitivity) {
            let q = plant(vec2f(nd, nb), min((level - avg) / max(p.sensitivity, 0.01), 1.0));
            nd = q.x;
            nb = q.y;
            hue = 1.0 - abs(1.0 - 2.0 * theta);
        }
        if (c.y == 0) {
            w = mix(avg, level, 0.15 / f32(substeps()));
        }
    }
    // Spores: a trickle of random seeds (3×3 blocks, big enough to take) near the centre, more
    // while the music is loud, so the tunnel keeps growing through passages with few onsets.
    if (c.y < 6 && p.spores > 0.0) {
        let block = vec2<u32>(vec2<i32>(c.x / 3, c.y / 3));
        let seed = u32(tick()) * 2654435761u ^ block.x * 40503u ^ block.y * 9973u ^ u32(m.seed * 16777216.0);
        if (rand01(seed) < p.spores * (0.3 + m.energy)) {
            let q = plant(vec2f(nd, nb), 1.0);
            nd = q.x;
            nb = q.y;
            hue = fract(theta * 2.0 + f32(tick()) * 0.01);
        }
    }
    return vec4f(clamp(nd, 0.0, 1.0), clamp(nb, 0.0, 1.0), hue, w);
}

// The grid at a fractional (theta, ring) position, linearly interpolated both ways.
fn smooth_at(t: f32, r: f32) -> vec4f {
    let t0 = floor(t - 0.5);
    let r0 = floor(r - 0.5);
    let f = vec2f(t - 0.5 - t0, r - 0.5 - r0);
    let i = vec2<i32>(i32(t0), i32(r0));
    let lo = mix(cell(i), cell(i + vec2<i32>(1, 0)), f.x);
    let hi = mix(cell(i + vec2<i32>(0, 1)), cell(i + vec2<i32>(1, 1)), f.x);
    return mix(lo, hi, f.y);
}

// The growth as a glossy relief: B is the height, lit from the vanishing point.
fn relief(u: f32, theta: f32, m: Music, p: Params) -> vec3f {
    let tu = fract(theta) * f32(grid().x);
    let s = smooth_at(tu, u);
    let e = 0.75;
    let du = smooth_at(tu, u + e).y - smooth_at(tu, u - e).y;
    let dt = smooth_at(tu + e, u).y - smooth_at(tu - e, u).y;
    let n = normalize(vec3f(-du * p.height, -dt * p.height, 1.0));
    let l = normalize(vec3f(-0.6, 0.35, 0.72));
    let diffuse = max(dot(n, l), 0.0);
    let spec = pow(max(reflect(-l, n).z, 0.0), 32.0) * p.gloss;
    let grow = smoothstep(0.08, 0.35, s.y);
    let col = palette(
        s.z * 0.7 + s.y * 0.8 + p.palette_shift,
        p.color_a,
        p.color_b,
        vec3f(1.0),
        vec3f(0.0, 0.33, 0.67),
    );
    // Bare substrate stays a dim, dark tissue; growth is bright and wet.
    let base = vec3f(0.02, 0.025, 0.04) * (0.5 + m.energy) * (0.4 + diffuse);
    var c = mix(base, col * (0.25 + 0.95 * diffuse) * (1.0 + p.glow), grow);
    c += vec3f(spec) * grow;
    return c;
}

fn scene(uv: vec2f, m: Music, p: Params) -> vec4f {
    let sway = p.tube * p.sway * vec2f(sin(m.motion * 0.19635), 0.6 * sin(m.motion * 0.0982 + 1.3));
    let hit3 = tube_hit(uv, 1.0, sway);
    let r = max(hit3.z, 1e-4);
    let rings = f32(grid().y);
    let depth = log(r / p.r_min) / log(p.r_max / p.r_min) * (rings - 1.0);
    // Glide: the flow of one step, spread over the step, so the tunnel streams smoothly.
    let per_step = p.flow * f32(substeps());
    let u = depth + per_step * (1.0 - tick_phase()) - p.kick_punch * pulse(hit(m), 8.0);
    let theta = hit3.y + 0.25 + p.twist * depth + p.spin * m.motion / 16.0;
    var c = relief(u, theta, m, p);
    let facing = tube_facing(r, 1.0);
    c *= mix(1.0, (0.3 + 1.4 * facing) * exp(-0.02 * hit3.x), p.tube);
    c *= smoothstep(p.r_min, p.r_min * 3.0, r) * (1.0 - smoothstep(rings - 6.0, rings, u));
    return vec4f(max(c, vec3f(0.0)), 0.85);
}
