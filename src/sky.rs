//! Night sky: gradient, twinkling stars and the occasional shooting star.

use rand::Rng;
use rand::rngs::StdRng;

use crate::palette::{self, Rgb, mix, scale};

pub struct Star {
    pub x: f32,
    pub y: f32,
    pub bright: f32,
    pub phase: f32,
    pub rate: f32,
    pub tint: Rgb,
}

pub struct Shooting {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
}

pub struct Sky {
    pub stars: Vec<Star>,
    pub shooting: Vec<Shooting>,
    next_shooting: f32,
    pub enabled: bool,
    /// Height of the distant skyline above the waterline, per column.
    shore: Vec<f32>,
    /// Warm windows in the skyline: (x, height above waterline, colour).
    lights: Vec<(f32, f32, Rgb)>,
    pub shore_enabled: bool,
}

impl Sky {
    pub fn new(w: usize, h: usize, rng: &mut StdRng) -> Self {
        let mut sky = Self {
            stars: Vec::new(),
            shooting: Vec::new(),
            next_shooting: 5.0,
            enabled: true,
            shore: Vec::new(),
            lights: Vec::new(),
            shore_enabled: true,
        };
        sky.resize(w, h, rng);
        sky
    }

    pub fn resize(&mut self, w: usize, h: usize, rng: &mut StdRng) {
        let horizon = h as f32 * 0.8;
        let count = ((w * h) / 95).clamp(24, 600);
        self.stars.clear();
        for _ in 0..count {
            // Bias stars away from the very bottom of the sky and the horizon haze.
            let y = rng.random::<f32>().powf(0.85) * horizon * 0.99;
            let b = 0.06 + 0.94 * rng.random::<f32>().powf(3.2);
            // A few stars are warm or blue rather than pure white.
            let tint = match rng.random::<f32>() {
                t if t < 0.12 => [1.0, 0.86, 0.72],
                t if t < 0.30 => [0.76, 0.86, 1.0],
                _ => [1.0, 1.0, 1.0],
            };
            self.stars.push(Star {
                x: rng.random::<f32>() * w as f32,
                y,
                bright: b,
                phase: rng.random::<f32>() * std::f32::consts::TAU,
                rate: 0.35 + rng.random::<f32>() * 2.2,
                tint,
            });
        }

        // A distant city on the far shore, a few cells tall.
        self.shore = vec![0.0; w];
        self.lights.clear();
        let mut x = 0.0f32;
        while x < w as f32 {
            let bw = rng.random_range(2.0..7.0);
            let bh: f32 = if rng.random_bool(0.14) {
                rng.random_range(3.0..6.5)
            } else {
                rng.random_range(0.6..2.2)
            };
            for xx in 0..bw as usize {
                let cx = x as usize + xx;
                if cx < w {
                    self.shore[cx] = bh.max(self.shore[cx]);
                }
            }
            let windows = (bw * bh / 16.0) as usize;
            for _ in 0..windows {
                let lx = x + rng.random::<f32>() * bw;
                let ly = 0.4 + rng.random::<f32>() * (bh - 0.8).max(0.1);
                let warm = rng.random::<f32>().powf(1.6);
                let c = mix([1.0, 0.62, 0.24], [1.0, 0.88, 0.6], warm);
                self.lights.push((lx, ly, scale(c, 0.4 + 0.6 * warm)));
            }
            x += bw + rng.random_range(0.0..2.5);
        }
    }

    pub fn update(&mut self, dt: f32, w: usize, h: usize, rng: &mut StdRng) {
        self.next_shooting -= dt;
        if self.next_shooting <= 0.0 {
            self.next_shooting = 7.0 + rng.random::<f32>() * 16.0;
            let from_left = rng.random_bool(0.5);
            let speed = 55.0 + rng.random::<f32>() * 55.0;
            let angle = 0.18 + rng.random::<f32>() * 0.35;
            self.shooting.push(Shooting {
                x: if from_left {
                    rng.random::<f32>() * w as f32 * 0.5
                } else {
                    w as f32 * (0.5 + rng.random::<f32>() * 0.5)
                },
                y: rng.random::<f32>() * h as f32 * 0.45,
                vx: if from_left { speed } else { -speed },
                vy: speed * angle,
                life: 0.6 + rng.random::<f32>() * 0.7,
                max_life: 1.0,
            });
            let n = self.shooting.len() - 1;
            self.shooting[n].max_life = self.shooting[n].life;
        }
        for s in &mut self.shooting {
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.life -= dt;
        }
        self.shooting.retain(|s| s.life > 0.0);
    }

    /// Paint stars into the background so it twinkles.
    pub fn paint(&self, bg: &mut [Rgb], w: usize, h: usize, horizon: f32, t: f32) {
        if !self.enabled {
            return;
        }
        for s in &self.stars {
            let tw = 0.62 + 0.38 * (t * s.rate + s.phase).sin();
            let b = s.bright * tw;
            let y = s.y as usize;
            if y >= h || s.y >= horizon {
                continue;
            }
            let x = s.x as usize;
            if x >= w {
                continue;
            }
            let c = scale(s.tint, b * 0.85);
            bg[y * w + x] = palette::add(bg[y * w + x], c);
            // Brighter stars get a faint halo so they read as points of light.
            if s.bright > 0.80 {
                let halo = scale(s.tint, b * 0.12);
                for (dx, dy) in [(-1i32, -1i32), (1, 1)] {
                    let xx = x as i32 + dx;
                    let yy = y as i32 + dy;
                    if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                        let i = yy as usize * w + xx as usize;
                        bg[i] = palette::add(bg[i], halo);
                    }
                }
            }
            // Reflection on the water, dimmed and shifted by the ripples.
            if horizon < h as f32 {
                let ry = 2.0 * horizon - s.y;
                if ry >= horizon && ry < h as f32 {
                    let jitter = (ry * 0.9 + t * 2.0).sin() * 1.6;
                    let rx = s.x + jitter;
                    if rx >= 0.0 && rx < (w - 1) as f32 {
                        let depth = (ry - horizon) / (h as f32 - horizon);
                        let k = b * 0.16 * (1.0 - depth * 0.6);
                        let i = ry as usize * w + rx as usize;
                        bg[i] = palette::add(bg[i], scale(s.tint, k));
                    }
                }
            }
        }

        if self.shore_enabled && horizon < h as f32 {
            let hz = horizon;
            // Light pollution: a warm glow climbing out of the city.
            let haze_top = (hz - 18.0).max(0.0) as usize;
            let span = (hz - haze_top as f32).max(1.0);
            for y in haze_top..(hz as usize).min(h) {
                let t = (y as f32 - haze_top as f32) / span;
                let k = t.powf(2.6) * 0.030;
                let c = [k, k * 0.55, k * 0.22];
                for x in 0..w {
                    let i = y * w + x;
                    bg[i] = palette::add(bg[i], c);
                }
            }

            for x in 0..self.shore.len().min(w) {
                let hgt = self.shore[x];
                let top = (hz - hgt).max(0.0) as usize;
                let bot = (hz as usize).min(h);
                for y in top..bot {
                    let i = y * w + x;
                    bg[i] = mix(bg[i], [0.008, 0.009, 0.016], 0.62);
                }
                // A faint rim light along the rooftops.
                if top < bot {
                    let i = top * w + x;
                    bg[i] = palette::add(bg[i], [0.020, 0.016, 0.012]);
                }
                // The city is mirrored on the water, squashed and wobbling.
                for k in 0..(hgt as usize).min(4) {
                    let y = (hz as usize + k).min(h - 1);
                    let wob = (y as f32 * 1.4 + t * 1.6).sin() * 1.2;
                    let xx = (x as f32 + wob).clamp(0.0, (w - 1) as f32) as usize;
                    let i = y * w + xx;
                    bg[i] = mix(bg[i], [0.006, 0.007, 0.013], 0.45);
                }
            }
            // Window lights, and their lazy reflections in the water.
            for (lx, ly, c) in &self.lights {
                let tw = 0.75 + 0.25 * (t * 0.7 + *lx * 0.13).sin();
                let x = *lx;
                let y = hz - ly;
                if x >= 0.0 && y >= 0.0 && x < (w - 1) as f32 && y < h as f32 {
                    let i = y as usize * w + x as usize;
                    bg[i] = palette::add(bg[i], scale(*c, 0.85 * tw));
                }
                let ry = hz + ly * 0.8;
                let rx = x + (ry * 0.6 + t * 1.4).sin() * 0.9;
                if rx >= 0.0 && ry < h as f32 && rx < (w - 1) as f32 {
                    let i = ry as usize * w + rx as usize;
                    bg[i] = palette::add(bg[i], scale(*c, 0.34 * tw));
                }
            }
        }

        for s in &self.shooting {
            let frac = (s.life / s.max_life).clamp(0.0, 1.0);
            let head = mix([1.0, 0.95, 0.85], [1.0, 0.7, 0.45], 1.0 - frac);
            let len = 9.0;
            let speed = (s.vx * s.vx + s.vy * s.vy).sqrt().max(1.0);
            let dx = s.vx / speed;
            let dy = s.vy / speed;
            for i in 0..14 {
                let k = i as f32 / 13.0;
                let x = s.x - dx * len * k;
                let y = s.y - dy * len * k;
                if x < 0.0 || y < 0.0 || x >= (w - 1) as f32 || y >= (h - 1) as f32 || y >= horizon
                {
                    continue;
                }
                let b = (1.0 - k).powf(2.2) * frac * 0.95;
                let c = scale(head, b);
                let i = y as usize * w + x as usize;
                bg[i] = palette::add(bg[i], c);
            }
        }
    }
}
