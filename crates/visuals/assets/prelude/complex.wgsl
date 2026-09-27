// Complex numbers as vec2 (x = real, y = imaginary).

fn cmul(a: vec2f, b: vec2f) -> vec2f {
    return vec2f(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn cdiv(a: vec2f, b: vec2f) -> vec2f {
    return vec2f(a.x * b.x + a.y * b.y, a.y * b.x - a.x * b.y) / max(dot(b, b), 1e-12);
}

fn csq(a: vec2f) -> vec2f {
    return vec2f(a.x * a.x - a.y * a.y, 2.0 * a.x * a.y);
}

fn cexp(a: vec2f) -> vec2f {
    return exp(a.x) * vec2f(cos(a.y), sin(a.y));
}

fn clog(a: vec2f) -> vec2f {
    return vec2f(log(max(length(a), 1e-12)), atan2(a.y, a.x));
}

fn cpow(a: vec2f, n: f32) -> vec2f {
    let r = pow(max(length(a), 1e-12), n);
    let t = atan2(a.y, a.x) * n;
    return r * vec2f(cos(t), sin(t));
}
