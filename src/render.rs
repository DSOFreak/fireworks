//! Turns the emissive buffer + sky into terminal cells.
//!
//! Each cell is an upper-half block: the foreground colour paints the top
//! sub-pixel and the background colour paints the bottom one, which gives us
//! square sub-pixels and twice the vertical resolution.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::fx::Emissive;
use crate::palette::{Rgb, luminance, mix, scale};
use crate::sky::Sky;

const SKY_TOP: Rgb = [0.012, 0.016, 0.038];
const SKY_MID: Rgb = [0.020, 0.030, 0.062];
const SKY_HORIZON: Rgb = [0.062, 0.056, 0.092];
const WATER_TOP: Rgb = [0.026, 0.036, 0.064];
const WATER_BOT: Rgb = [0.002, 0.004, 0.010];

const BAYER: [[f32; 4]; 4] = [
    [0.0, 8.0, 2.0, 10.0],
    [12.0, 4.0, 14.0, 6.0],
    [3.0, 11.0, 1.0, 9.0],
    [15.0, 7.0, 13.0, 5.0],
];

pub struct Renderer {
    pub w: usize,
    pub h: usize,
    pub sw: usize,
    pub sh: usize,
    grad: Vec<Rgb>,
    comp: Vec<Rgb>,
    bloom: Vec<Rgb>,
    tmp: Vec<Rgb>,
    bw: usize,
    bh: usize,
    block: usize,
    pub exposure: f32,
    pub bloom_strength: f32,
    /// Ordered dithering hides banding but lights up almost every pixel each
    /// frame, which costs a lot in a GIF. Off for recording.
    pub dither: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
            sw: 0,
            sh: 0,
            grad: Vec::new(),
            comp: Vec::new(),
            bloom: Vec::new(),
            tmp: Vec::new(),
            bw: 0,
            bh: 0,
            block: 4,
            exposure: 1.5,
            bloom_strength: 0.35,
            dither: true,
        }
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn horizon(&self) -> f32 {
        (self.sh as f32 * 0.80).floor()
    }

    pub fn ensure(&mut self, w: usize, h: usize) {
        let (w, h) = (w.max(1), h.max(1));
        if w == self.w && h == self.h {
            return;
        }
        self.w = w;
        self.h = h;
        self.sw = w;
        self.sh = h * 2;
        let hz = self.horizon();
        self.grad = Vec::with_capacity(self.sw * self.sh);
        for y in 0..self.sh {
            let yf = y as f32;
            let c = if yf < hz {
                let t = (yf / hz.max(1.0)).clamp(0.0, 1.0);
                if t < 0.62 {
                    mix(SKY_TOP, SKY_MID, (t / 0.62).powf(0.85))
                } else {
                    mix(SKY_MID, SKY_HORIZON, ((t - 0.62) / 0.38).powf(1.6))
                }
            } else {
                let t = ((yf - hz) / (self.sh as f32 - hz).max(1.0)).clamp(0.0, 1.0);
                mix(WATER_TOP, WATER_BOT, t.powf(0.75))
            };
            // A soft airglow sitting on the waterline.
            let glow = (-((yf - hz) / 11.0).powi(2)).exp() * 0.014;
            let band = [glow * 1.0, glow * 0.78, glow * 1.25];
            // Two very wide, very dim nebula glows give the sky depth.
            let nebula = [[0.030, 0.016, 0.052], [0.008, 0.026, 0.044]];
            let spots = [(0.24f32, 0.30f32, 0.42f32), (0.78f32, 0.52f32, 0.5f32)];
            for x in 0..self.sw {
                let xf = x as f32 / self.sw.max(1) as f32;
                let yf = (yf / self.sh.max(1) as f32).min(1.0);
                let mut neb = [0.0f32; 3];
                for (i, (sx, sy, r)) in spots.iter().enumerate() {
                    let dx = (xf - sx) / r;
                    let dy = (yf - sy) / (r * 0.7);
                    let d2 = dx * dx + dy * dy;
                    if d2 < 1.0 {
                        let f = (1.0 - d2).powi(2);
                        for ch in 0..3 {
                            neb[ch] += nebula[i][ch] * f;
                        }
                    }
                }
                self.grad.push([
                    c[0] + band[0] + neb[0],
                    c[1] + band[1] + neb[1],
                    c[2] + band[2] + neb[2],
                ]);
            }
        }
        self.comp = vec![[0.0; 3]; self.sw * self.sh];
        // Keep the glow radius constant in *relative* screen terms.
        self.block = (self.sw / 48).max(2);
        self.bw = self.sw.div_ceil(self.block);
        self.bh = self.sh.div_ceil(self.block);
        self.bloom = vec![[0.0; 3]; self.bw * self.bh];
        self.tmp = vec![[0.0; 3]; self.bw * self.bh];
    }

    /// Build the full colour field: sky, water, fireworks, reflections, bloom.
    pub fn compose(&mut self, em: &Emissive, sky: &Sky, t: f32, water: bool) {
        if self.comp.len() != self.grad.len() {
            return;
        }
        self.comp.copy_from_slice(&self.grad);
        let hz = self.horizon();
        sky.paint(&mut self.comp, self.sw, self.sh, hz, t);

        // Fireworks on top of the sky.
        for (dst, src) in self.comp.iter_mut().zip(em.px.iter()) {
            dst[0] += src[0];
            dst[1] += src[1];
            dst[2] += src[2];
        }

        self.update_bloom(em);

        // Water: mirrored, rippled and dimmed.
        if water && hz + 1.0 < self.sh as f32 {
            let hzi = hz as usize;
            for y in hzi..self.sh {
                let depth = (y as f32 - hz) / (self.sh as f32 - hz).max(1.0);
                let amp = 0.6 + depth * 2.2;
                let wob = (y as f32 * 0.55 + t * 1.9).sin() * amp
                    + (y as f32 * 0.19 - t * 0.8).sin() * amp * 0.55;
                let sy = 2.0 * hz - y as f32;
                if sy < 0.0 {
                    continue;
                }
                let row = y * self.sw;
                let fade = 0.66 * (1.0 - depth * 0.4);
                for x in 0..self.sw {
                    let sx = x as f32 + wob;
                    if sx < 0.0 || sx >= (self.sw - 1) as f32 {
                        continue;
                    }
                    let c = em.sample(sx, sy);
                    let i = row + x;
                    self.comp[i][0] += c[0] * fade;
                    self.comp[i][1] += c[1] * fade;
                    self.comp[i][2] += c[2] * fade;
                }
                // Faint moonlit ripples so the water reads as water.
                let shimmer =
                    (0.5 + 0.5 * (y as f32 * 1.9 - t * 1.3).sin()) * (1.0 - depth) * 0.018;
                for x in 0..self.sw {
                    let wob = 0.6 + 0.4 * ((x as f32 * 0.07 + y as f32 * 0.4 + t).sin());
                    let i = row + x;
                    self.comp[i][0] += shimmer * wob * 0.55;
                    self.comp[i][1] += shimmer * wob * 0.75;
                    self.comp[i][2] += shimmer * wob * 1.1;
                }
            }
        }

        // Add the blurred light spill.
        if self.bloom_strength > 0.0 {
            let inv = 1.0 / self.block as f32;
            for y in 0..self.sh {
                let by = (y as f32 * inv) - 0.5;
                for x in 0..self.sw {
                    let bx = (x as f32 * inv) - 0.5;
                    let c = self.sample_bloom(bx, by);
                    let i = y * self.sw + x;
                    self.comp[i][0] += c[0] * self.bloom_strength;
                    self.comp[i][1] += c[1] * self.bloom_strength;
                    self.comp[i][2] += c[2] * self.bloom_strength;
                }
            }
        }
    }

    fn update_bloom(&mut self, em: &Emissive) {
        for by in 0..self.bh {
            for bx in 0..self.bw {
                let mut acc = [0.0f32; 3];
                let mut n = 0.0;
                let b = self.block;
                for y in (by * b)..((by + 1) * b).min(self.sh) {
                    for x in (bx * b)..((bx + 1) * b).min(self.sw) {
                        let c = em.px[y * em.w + x];
                        acc[0] += c[0];
                        acc[1] += c[1];
                        acc[2] += c[2];
                        n += 1.0;
                    }
                }
                let inv = if n > 0.0 { 1.0 / n } else { 0.0 };
                let c = scale(acc, inv);
                // Soft bright pass: dim trails don't smear the whole screen.
                let l = luminance(c);
                let k = if l > 0.55 {
                    ((l - 0.55) / l.max(1e-5)) * 0.95
                } else {
                    0.0
                };
                self.bloom[by * self.bw + bx] = scale(c, k);
            }
        }

        // Two cheap separable blurs make the glow wide and smooth.
        for _ in 0..2 {
            for y in 0..self.bh {
                for x in 0..self.bw {
                    let i = y * self.bw + x;
                    let a = self.bloom[i - if x > 0 { 1 } else { 0 }];
                    let b = self.bloom[i];
                    let c = self.bloom[i + if x + 1 < self.bw { 1 } else { 0 }];
                    self.tmp[i] = [
                        a[0] * 0.25 + b[0] * 0.5 + c[0] * 0.25,
                        a[1] * 0.25 + b[1] * 0.5 + c[1] * 0.25,
                        a[2] * 0.25 + b[2] * 0.5 + c[2] * 0.25,
                    ];
                }
            }
            std::mem::swap(&mut self.bloom, &mut self.tmp);
            for y in 0..self.bh {
                for x in 0..self.bw {
                    let i = y * self.bw + x;
                    let a = self.bloom[i - if y > 0 { self.bw } else { 0 }];
                    let b = self.bloom[i];
                    let c = self.bloom[i + if y + 1 < self.bh { self.bw } else { 0 }];
                    self.tmp[i] = [
                        a[0] * 0.25 + b[0] * 0.5 + c[0] * 0.25,
                        a[1] * 0.25 + b[1] * 0.5 + c[1] * 0.25,
                        a[2] * 0.25 + b[2] * 0.5 + c[2] * 0.25,
                    ];
                }
            }
            std::mem::swap(&mut self.bloom, &mut self.tmp);
        }
    }

    fn sample_bloom(&self, x: f32, y: f32) -> Rgb {
        let x = x.clamp(0.0, (self.bw - 1) as f32);
        let y = y.clamp(0.0, (self.bh - 1) as f32);
        let x0 = x as usize;
        let y0 = y as usize;
        let x1 = (x0 + 1).min(self.bw - 1);
        let y1 = (y0 + 1).min(self.bh - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let a = mix(
            self.bloom[y0 * self.bw + x0],
            self.bloom[y0 * self.bw + x1],
            fx,
        );
        let b = mix(
            self.bloom[y1 * self.bw + x0],
            self.bloom[y1 * self.bw + x1],
            fx,
        );
        mix(a, b, fy)
    }

    /// Final grade: exposure, filmic roll-off, vignette and dithering.
    ///
    /// The tone curve is applied to luminance and the chroma is carried along,
    /// so a dense cluster of gold sparks stays gold instead of bleaching white.
    #[inline]
    fn grade(&self, c: Rgb, x: usize, y: usize) -> Color {
        let nx = (x as f32 / (self.sw - 1).max(1) as f32) * 2.0 - 1.0;
        let ny = (y as f32 / (self.sh - 1).max(1) as f32) * 2.0 - 1.0;
        let vig = 1.0 - 0.26 * (nx * nx * 0.55 + ny * ny * 0.45) - 0.04;
        let d = if self.dither {
            (BAYER[y & 3][x & 3] / 16.0 - 0.5) / 255.0
        } else {
            0.0
        };
        let k = self.exposure * vig;
        let v = [c[0] * k, c[1] * k, c[2] * k];
        let l = luminance(v);
        let mapped = 1.0 - (-l).exp();
        let gain = if l > 1e-5 { mapped / l } else { 0.0 };
        let mut out = [0u8; 3];
        let mut lin = [v[0] * gain, v[1] * gain, v[2] * gain];
        // A touch of extra chroma keeps the show from looking dusty.
        let ol = luminance(lin);
        for ch in &mut lin {
            *ch = ol + (*ch - ol) * 1.22;
        }
        for ch in 0..3 {
            let q = (lin[ch] + d).clamp(0.0, 1.0);
            out[ch] = (q * 255.0 + 0.5) as u8;
        }
        Color::Rgb(out[0], out[1], out[2])
    }

    pub fn blit(&self, buf: &mut Buffer, area: Rect) {
        if self.comp.len() != self.sw * self.sh {
            return;
        }
        let w = area.width.min(self.sw as u16);
        for cy in 0..area.height.min(self.h as u16) {
            let top_row = cy as usize * 2;
            for cx in 0..w {
                let x = cx as usize;
                let top = self.grade(self.comp[top_row * self.sw + x], x, top_row);
                let bot = self.grade(self.comp[(top_row + 1) * self.sw + x], x, top_row + 1);
                if let Some(cell) = buf.cell_mut((area.x + cx, area.y + cy)) {
                    cell.set_symbol("▀");
                    cell.fg = top;
                    cell.bg = bot;
                }
            }
        }
    }
}
