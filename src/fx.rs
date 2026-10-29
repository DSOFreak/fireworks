//! Additive, fading framebuffer that every spark is drawn into.
//!
//! The buffer is never cleared between frames - it is multiplied by a decay
//! factor instead. That single trick gives every particle glowing comet
//! trails and makes the whole thing feel photographic.

use crate::palette::{Rgb, scale};

pub struct Emissive {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Emissive {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w: w.max(1),
            h: h.max(1),
            px: vec![[0.0; 3]; w.max(1) * h.max(1)],
        }
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        let (w, h) = (w.max(1), h.max(1));
        if w == self.w && h == self.h {
            return;
        }
        self.w = w;
        self.h = h;
        self.px = vec![[0.0; 3]; w * h];
    }

    /// Multiply the whole buffer by `f`. Called once per frame.
    pub fn fade(&mut self, f: f32) {
        for p in &mut self.px {
            p[0] *= f;
            p[1] *= f;
            p[2] *= f;
        }
    }

    #[inline]
    pub fn add(&mut self, x: f32, y: f32, c: Rgb, w: f32) {
        if !(0.0..self.w as f32).contains(&x) || !(0.0..self.h as f32).contains(&y) {
            return;
        }
        let i = y as usize * self.w + x as usize;
        let p = &mut self.px[i];
        p[0] += c[0] * w;
        p[1] += c[1] * w;
        p[2] += c[2] * w;
    }

    /// A single point of light with a soft 3x3 falloff around it.
    #[inline]
    pub fn splat(&mut self, x: f32, y: f32, c: Rgb, w: f32) {
        self.add(x, y, c, w);
        let s = w * 0.22;
        self.add(x - 1.0, y, c, s);
        self.add(x + 1.0, y, c, s);
        self.add(x, y - 1.0, c, s);
        self.add(x, y + 1.0, c, s);
        self.add(x - 1.0, y - 1.0, c, s * 0.45);
        self.add(x + 1.0, y - 1.0, c, s * 0.45);
        self.add(x - 1.0, y + 1.0, c, s * 0.45);
        self.add(x + 1.0, y + 1.0, c, s * 0.45);
    }

    /// Draw a motion streak from a particle's previous to current position.
    /// The head is brighter than the tail so motion reads clearly at any fps.
    pub fn streak(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgb, w: f32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let steps = (dist.ceil() as usize).clamp(1, 28);
        let inv = 1.0 / steps as f32;
        for i in 0..=steps {
            let t = i as f32 * inv;
            let k = w * (0.3 + 0.7 * t);
            self.splat(x0 + dx * t, y0 + dy * t, c, k * 0.55);
        }
    }

    /// Round soft glow, used for muzzle flashes and burst blooms.
    pub fn glow(&mut self, x: f32, y: f32, radius: f32, c: Rgb, power: f32) {
        if radius <= 0.0 || power <= 0.0 {
            return;
        }
        let r = radius.max(1.0);
        let ri = r.ceil() as i32;
        let x0 = (x - r).floor() as i32;
        let y0 = (y - r).floor() as i32;
        let inv = 1.0 / (r * r);
        for yy in y0..=y0 + ri * 2 {
            for xx in x0..=x0 + ri * 2 {
                let dx = xx as f32 - x;
                let dy = yy as f32 - y;
                let d2 = (dx * dx + dy * dy) * inv;
                if d2 >= 1.0 {
                    continue;
                }
                let f = (1.0 - d2) * (1.0 - d2);
                self.add(xx as f32, yy as f32, c, power * f);
            }
        }
    }

    /// Fade everything a touch and keep the buffer from accumulating infinities.
    pub fn decay_clamp(&mut self, ceiling: f32) {
        for p in &mut self.px {
            let l = p[0].max(p[1]).max(p[2]);
            if l > ceiling {
                *p = scale(*p, ceiling / l);
            }
        }
    }

    pub fn sample(&self, x: f32, y: f32) -> Rgb {
        if x < 0.0 || y < 0.0 || x >= self.w as f32 - 1.0 || y >= self.h as f32 - 1.0 {
            return [0.0; 3];
        }
        let x0 = x as usize;
        let y0 = y as usize;
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;
        let i = y0 * self.w + x0;
        let a = self.px[i];
        let b = self.px[i + 1];
        let c = self.px[i + self.w];
        let d = self.px[i + self.w + 1];
        let top = crate::palette::mix(a, b, fx);
        let bot = crate::palette::mix(c, d, fx);
        crate::palette::mix(top, bot, fy)
    }
}
