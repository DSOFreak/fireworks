//! Colour helpers and the firework colour palettes.

pub type Rgb = [f32; 3];

pub const WHITE: Rgb = [1.0, 1.0, 1.0];

/// A warm ember tone that sparks cool down into.
pub const EMBER: Rgb = [1.0, 0.42, 0.09];

#[inline]
pub fn add(a: Rgb, b: Rgb) -> Rgb {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub fn scale(a: Rgb, s: f32) -> Rgb {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[inline]
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[inline]
pub fn luminance(c: Rgb) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

const fn c(hex: u32) -> Rgb {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

/// A palette is a looping gradient that shells sample colours from.
pub struct Palette {
    pub name: &'static str,
    pub stops: &'static [Rgb],
}

impl Palette {
    /// Sample the gradient. `t` wraps, so anything goes.
    pub fn at(&self, t: f32) -> Rgb {
        let n = self.stops.len();
        if n == 0 {
            return WHITE;
        }
        let t = t.rem_euclid(1.0) * n as f32;
        let i = (t.floor() as usize) % n;
        let f = t - t.floor();
        mix(self.stops[i], self.stops[(i + 1) % n], f)
    }
}

static CLASSIC: [Rgb; 5] = [
    c(0xffb628),
    c(0xff2f2f),
    c(0x2bff7a),
    c(0x2f8cff),
    c(0xfff3dd),
];
static EMBER_P: [Rgb; 4] = [c(0xff2a0a), c(0xff7a00), c(0xffc233), c(0xffeec2)];
static OCEAN: [Rgb; 4] = [c(0x00e5ff), c(0x1a6bff), c(0xaef4ff), c(0x7a3bff)];
static AURORA: [Rgb; 4] = [c(0x22ffb0), c(0x2a90ff), c(0xa855ff), c(0xf28cff)];
static CANDY: [Rgb; 4] = [c(0xff4fb8), c(0xff9bdd), c(0xb47dff), c(0x7fe2ff)];
static NEON: [Rgb; 4] = [c(0x2bff00), c(0x00eaff), c(0xff00cc), c(0xf2ff00)];
static GOLD: [Rgb; 4] = [c(0xffdf9a), c(0xffbb33), c(0xff8c1f), c(0xfff6dd)];
static RAINBOW: [Rgb; 6] = [
    c(0xff3030),
    c(0xff9a30),
    c(0xffe630),
    c(0x40ff70),
    c(0x30b0ff),
    c(0xb850ff),
];

pub static PALETTES: [Palette; 8] = [
    Palette {
        name: "classic",
        stops: &CLASSIC,
    },
    Palette {
        name: "ember",
        stops: &EMBER_P,
    },
    Palette {
        name: "ocean",
        stops: &OCEAN,
    },
    Palette {
        name: "aurora",
        stops: &AURORA,
    },
    Palette {
        name: "candy",
        stops: &CANDY,
    },
    Palette {
        name: "neon",
        stops: &NEON,
    },
    Palette {
        name: "gold",
        stops: &GOLD,
    },
    Palette {
        name: "rainbow",
        stops: &RAINBOW,
    },
];

pub fn find_palette(name: &str) -> Option<usize> {
    if name.eq_ignore_ascii_case("random") {
        return None;
    }
    PALETTES
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn palette_names() -> String {
    let mut names = vec!["random"];
    names.extend(PALETTES.iter().map(|p| p.name));
    names.join(", ")
}
