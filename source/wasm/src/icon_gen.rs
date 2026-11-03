use {
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::shared::QualifiedChannelId,
    spaghettinuum::interface::identity::Identity,
    std::{
        f64::consts::PI,
        fmt::Write,
    },
};

const SIZE: f64 = 100.;
const CORNER: f64 = 20.;
const STROKE_CM: f64 = 0.072;
const HUE_COUNT: usize = 10;
const CHROMA: f64 = 0.07;
const LIGHTNESS_LIGHT_THEME: [f64; 2] = [0.45, 0.62];

struct Rng(u64);

impl Rng {
    fn new(domain: &str, data: &[u8]) -> Rng {
        let mut hash = Sha256::new();
        hash.update(domain.as_bytes());
        hash.update([0u8]);
        hash.update(data);
        let digest = hash.finalize();
        return Rng(u64::from_le_bytes(digest[0 .. 8].try_into().unwrap()));
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        return z ^ (z >> 31);
    }

    fn unit(&mut self) -> f64 {
        return (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        return low + (high - low) * self.unit();
    }

    fn index(&mut self, count: usize) -> usize {
        return (self.next_u64() % count as u64) as usize;
    }
}

fn srgb_gamma(v: f64) -> u8 {
    let v = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    };
    return (v * 255.).round() as u8;
}

fn oklch_hex(l: f64, c: f64, hue_deg: f64) -> String {
    let h = hue_deg.to_radians();
    let a = c * h.cos();
    let b = c * h.sin();
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    let r = 4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_;
    let g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_;
    let b = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_;
    return format!("#{:02x}{:02x}{:02x}", srgb_gamma(r), srgb_gamma(g), srgb_gamma(b));
}

fn svg_data_url(rng: &mut Rng, outline: &str, body: &str) -> String {
    let hue = 30. + rng.index(HUE_COUNT) as f64 * 360. / HUE_COUNT as f64;
    let band = rng.index(LIGHTNESS_LIGHT_THEME.len());
    let light = oklch_hex(LIGHTNESS_LIGHT_THEME[band], CHROMA, hue);
    let dark = oklch_hex([0.72, 0.86][band], CHROMA, hue);
    let svg =
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {SIZE} {SIZE}"><style>g{{fill:none;stroke:{dark};stroke-width:{STROKE_CM}cm;stroke-linecap:round}}.o{{stroke-width:{outline_width}cm}}g>*{{vector-effect:non-scaling-stroke}}@media (prefers-color-scheme: light){{g{{stroke:{light}}}}}</style><clipPath id="c">{outline}</clipPath><g clip-path="url(#c)"><g class="o">{outline}</g>{body}</g></svg>"#,
            outline_width = STROKE_CM * 2.
        );
    return format!("data:image/svg+xml,{}", urlencoding::encode(&svg));
}

fn edge_point(rng: &mut Rng, side: usize) -> (f64, f64) {
    let t = rng.range(CORNER, SIZE - CORNER);
    match side {
        0 => return (t, 0.),
        1 => return (SIZE, t),
        2 => return (t, SIZE),
        3 => return (0., t),
        _ => unreachable!(),
    }
}

fn box_outline() -> String {
    return format!(r#"<rect width="{SIZE}" height="{SIZE}" rx="{CORNER}"/>"#);
}

pub fn channel_icon(id: &QualifiedChannelId) -> String {
    let mut data = id.identity.to_bytes();
    data.extend_from_slice(&id.channel.0.to_le_bytes());
    let mut rng = Rng::new("kwa-channel-icon", &data);
    let outline = box_outline();
    let mut body = String::new();
    for _ in 0 .. 1 + rng.index(5) {
        let (start, end) = loop {
            let side_a = rng.index(4);
            let side_b = (side_a + 1 + rng.index(3)) % 4;
            let a = edge_point(&mut rng, side_a);
            let b = edge_point(&mut rng, side_b);
            if (a.0 - b.0).hypot(a.1 - b.1) >= 40. {
                break (a, b);
            }
        };
        write!(
            body,
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/>"#,
            start.0,
            start.1,
            end.0,
            end.1
        ).unwrap();
    }
    return svg_data_url(&mut rng, &outline, &body);
}

pub fn identity_icon(id: &Identity) -> String {
    let mut rng = Rng::new("kwa-identity-icon", &id.to_bytes());
    let center = SIZE / 2.;
    let outer_radius = center;
    let mut body = String::new();
    let circle = |body: &mut String, x: f64, y: f64, r: f64| {
        write!(body, r#"<circle cx="{x:.2}" cy="{y:.2}" r="{r:.2}"/>"#).unwrap();
    };
    circle(&mut body, center, center, outer_radius);
    for i in 0 .. 1 + rng.index(3) {
        let radius = rng.range(18., outer_radius * 0.7);
        let dist = if i == 0 {
            outer_radius - radius
        } else {
            (outer_radius - radius - 13.5) * rng.unit().sqrt()
        };
        let angle = rng.range(0., 2. * PI);
        circle(&mut body, center + dist * angle.cos(), center + dist * angle.sin(), radius);
    }
    return svg_data_url(&mut rng, &box_outline(), &body);
}
