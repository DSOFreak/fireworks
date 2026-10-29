//! Rockets, shells and sparks - the pyrotechnics themselves.

use std::f32::consts::TAU;

use rand::Rng;
use rand::rngs::StdRng;

use crate::fx::Emissive;
use crate::palette::{EMBER, PALETTES, Rgb, WHITE, mix};
use crate::text::{Glyph, Launch, Message};

pub const GRAV: f32 = 42.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Peony,
    Chrysanthemum,
    Willow,
    Ring,
    Palm,
    Crossette,
    Strobe,
    Heart,
    Spiral,
    /// A shell that spells one character.
    Letter(char),
}

impl Kind {
    pub fn from_name(name: &str) -> Option<Kind> {
        let all = [
            Kind::Peony,
            Kind::Chrysanthemum,
            Kind::Willow,
            Kind::Ring,
            Kind::Palm,
            Kind::Crossette,
            Kind::Strobe,
            Kind::Heart,
            Kind::Spiral,
        ];
        all.into_iter()
            .find(|k| k.label().eq_ignore_ascii_case(name.trim()))
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Peony => "peony",
            Kind::Chrysanthemum => "chrysanthemum",
            Kind::Willow => "willow",
            Kind::Ring => "ring",
            Kind::Palm => "palm",
            Kind::Crossette => "crossette",
            Kind::Strobe => "strobe",
            Kind::Heart => "heart",
            Kind::Spiral => "spiral",
            Kind::Letter(_) => "letter",
        }
    }
}

const TABLE: [(Kind, f32); 9] = [
    (Kind::Peony, 3.4),
    (Kind::Chrysanthemum, 2.5),
    (Kind::Willow, 1.0),
    (Kind::Ring, 1.0),
    (Kind::Palm, 1.0),
    (Kind::Crossette, 1.1),
    (Kind::Strobe, 0.9),
    (Kind::Heart, 0.6),
    (Kind::Spiral, 0.9),
];

#[derive(Clone, Copy)]
pub struct Shell {
    pub kind: Kind,
    pub a: Rgb,
    pub b: Rgb,
    pub speed: f32,
    /// Sub-pixels per font dot, for letter shells.
    pub scale: f32,
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: Rgb,
    pub life: f32,
    pub max_life: f32,
    pub gravity: f32,
    pub drag: f32,
    pub bright: f32,
    pub flicker_rate: f32,
    pub flicker_phase: f32,
    pub flicker_amt: f32,
    pub glitter: f32,
    pub split: u8,
    pub split_at: f32,
    pub split_speed: f32,
    pub split_color: Rgb,
    pub split_done: bool,
    pub cooling: bool,
    /// Crisp sparks (rings) are drawn as points rather than motion streaks.
    pub trail: bool,
}

impl Particle {
    fn new(x: f32, y: f32, vx: f32, vy: f32, color: Rgb, life: f32) -> Self {
        Self {
            x,
            y,
            px: x,
            py: y,
            vx,
            vy,
            color,
            life,
            max_life: life,
            gravity: GRAV,
            drag: 0.55,
            bright: 1.0,
            flicker_rate: 0.0,
            flicker_phase: 0.0,
            flicker_amt: 0.0,
            glitter: 0.0,
            split: 0,
            split_at: 0.55,
            split_speed: 12.0,
            split_color: color,
            split_done: true,
            cooling: true,
            trail: true,
        }
    }
}

pub struct Rocket {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub burst_y: f32,
    pub shell: Shell,
    pub wobble: f32,
    pub wobble_phase: f32,
    pub life: f32,
    pub color: Rgb,
}

pub struct Flash {
    pub x: f32,
    pub y: f32,
    pub life: f32,
    pub max_life: f32,
    pub radius: f32,
    pub color: Rgb,
    pub power: f32,
}

pub struct Fireworks {
    pub particles: Vec<Particle>,
    pub rockets: Vec<Rocket>,
    pub flashes: Vec<Flash>,
    pub rng: StdRng,
    pub density: f32,
    pub next_launch: f32,
    pub queue: Vec<(f32, Shell)>,
    pub forced_palette: Option<usize>,
    pub forced_kind: Option<Kind>,
    pub w: f32,
    pub h: f32,
    pub count_scale: f32,
    pub size_scale: f32,
    pub total_launched: u64,
    pub glyphs: Vec<Glyph>,
    pub message: Option<Message>,
}

impl Fireworks {
    pub fn new(
        w: f32,
        h: f32,
        mut rng: StdRng,
        density: f32,
        forced_palette: Option<usize>,
        forced_kind: Option<Kind>,
        text: Option<&str>,
    ) -> Self {
        let count_scale = ((w * h) / (160.0 * 100.0)).clamp(0.35, 1.8);
        let message = text.and_then(|t| {
            let palette = forced_palette.unwrap_or_else(|| rng.random_range(0..PALETTES.len()));
            Message::new(t, palette, &mut rng)
        });
        Self {
            particles: Vec::new(),
            rockets: Vec::new(),
            flashes: Vec::new(),
            rng,
            density,
            next_launch: 0.55,
            queue: Vec::new(),
            forced_palette,
            forced_kind,
            w,
            h,
            count_scale,
            size_scale: ((w * h) / (160.0 * 96.0)).sqrt().clamp(0.6, 1.6),
            total_launched: 0,
            glyphs: Vec::new(),
            message,
        }
    }

    pub fn resize(&mut self, w: f32, h: f32) {
        let sx = w / self.w.max(1.0);
        let sy = h / self.h.max(1.0);
        for p in &mut self.particles {
            p.x *= sx;
            p.px *= sx;
            p.y *= sy;
            p.py *= sy;
        }
        for r in &mut self.rockets {
            r.x *= sx;
            r.y *= sy;
            r.burst_y *= sy;
        }
        for f in &mut self.flashes {
            f.x *= sx;
            f.y *= sy;
        }
        self.w = w;
        self.h = h;
        self.count_scale = ((w * h) / (160.0 * 100.0)).clamp(0.35, 1.8);
        self.size_scale = ((w * h) / (160.0 * 96.0)).sqrt().clamp(0.6, 1.6);
    }

    fn pick_kind(&mut self) -> Kind {
        if let Some(k) = self.forced_kind {
            return k;
        }
        let total: f32 = TABLE.iter().map(|(_, w)| w).sum();
        let mut r = self.rng.random::<f32>() * total;
        for (k, w) in TABLE {
            r -= w;
            if r <= 0.0 {
                return k;
            }
        }
        Kind::Peony
    }

    fn pick_palette(&mut self) -> usize {
        if let Some(p) = self.forced_palette {
            return p;
        }
        // "classic" is the house style, so give it a little extra weight.
        let mut r = self.rng.random::<f32>() * (PALETTES.len() as f32 + 2.0);
        if r < 3.0 {
            return 0;
        }
        r -= 3.0;
        (r as usize).min(PALETTES.len() - 1)
    }

    fn shell(&mut self, kind: Kind) -> Shell {
        let pal = self.pick_palette();
        let stops = &PALETTES[pal];
        let a = stops.at(self.rng.random::<f32>());
        let b = if self.rng.random_bool(0.62) {
            stops.at(self.rng.random::<f32>())
        } else {
            mix(a, WHITE, 0.45)
        };
        let mut speed = (25.0 + self.rng.random::<f32>() * 21.0) * self.size_scale;
        if kind == Kind::Willow {
            speed *= 0.85;
        }
        Shell {
            kind,
            a,
            b,
            speed,
            scale: 1.0,
        }
    }

    fn shell_color(&mut self, shell: &Shell) -> Rgb {
        let base = if self.rng.random_bool(0.62) {
            shell.a
        } else {
            shell.b
        };
        let hot = self.rng.random::<f32>() * 0.22;
        mix(base, WHITE, hot)
    }

    fn schedule(&mut self) {
        let base = 0.5 + self.rng.random::<f32>() * 1.25;
        self.next_launch = (base / self.density.max(0.05)).max(0.12);
        if self.message.is_some() {
            // The message is the whole show: no random shells to read through.
            self.next_launch = f32::MAX / 4.0;
            return;
        }
        let roll = self.rng.random::<f32>();
        if roll < 0.11 {
            // A short finale: a salvo of shells one after another.
            let n = self.rng.random_range(5..10);
            let mut delay = 0.0;
            for _ in 0..n {
                let kind = self.pick_kind();
                let shell = self.shell(kind);
                self.queue.push((delay, shell));
                delay += self.rng.random_range(0.06..0.26);
            }
            self.next_launch = (base / self.density.max(0.05) + 1.1).max(1.0);
        } else if roll < 0.24 {
            // A twin shell: two different styles side by side.
            let kind = self.pick_kind();
            let shell = self.shell(kind);
            self.launch(shell);
            let second = self.pick_kind();
            let other = Shell {
                kind: second,
                a: shell.b,
                b: shell.a,
                speed: shell.speed * 0.92,
                scale: 1.0,
            };
            self.queue.push((self.rng.random_range(0.06..0.2), other));
        } else {
            let kind = self.pick_kind();
            let shell = self.shell(kind);
            self.launch(shell);
        }
    }

    /// Fire a shell right now (used by the `f` key and by the scheduler).
    pub fn launch(&mut self, shell: Shell) {
        self.launch_in(shell, 0.17, 0.45);
    }

    /// Fire a shell at a random spot inside a band of the sky.
    fn launch_in(&mut self, shell: Shell, top: f32, bottom: f32) {
        let x = self.w * self.rng.random_range(0.22..0.78);
        let target_y = self.h * self.rng.random_range(top..bottom);
        let drift = self.rng.random_range(-0.13..0.13) * self.w;
        let target_x = (x + drift).clamp(self.w * 0.28, self.w * 0.72);
        let wobble = self.rng.random_range(0.5..2.5);
        self.launch_shell(shell, x, target_x, target_y, wobble);
    }

    /// Send a letter shell to an exact spot so it lands in the layout.
    pub fn launch_letter(&mut self, letter: Launch) {
        let shell = Shell {
            kind: Kind::Letter(letter.ch),
            a: letter.color,
            b: letter.color,
            speed: 26.0,
            scale: letter.slot.scale,
        };
        let x = (letter.slot.x + self.rng.random_range(-0.05..0.05) * self.w)
            .clamp(self.w * 0.05, self.w * 0.95);
        self.launch_shell(shell, x, letter.slot.x, letter.slot.y, 0.0);
    }

    fn launch_shell(&mut self, shell: Shell, x: f32, target_x: f32, target_y: f32, wobble: f32) {
        // Launch from the far shore, just under the waterline.
        let y = self.h * 0.80;
        let height = (y - target_y).max(10.0);
        let vy = -(2.0 * GRAV * height).sqrt() * self.rng.random_range(0.97..1.05);
        let t_apex = (-vy / GRAV).max(0.2);
        let vx = (target_x - x) / t_apex;

        let color = mix(shell.a, WHITE, 0.25);
        self.rockets.push(Rocket {
            x,
            y,
            vx,
            vy,
            burst_y: target_y,
            shell,
            wobble,
            wobble_phase: self.rng.random::<f32>() * TAU,
            life: t_apex * 1.35,
            color,
        });
        // Muzzle flash at the launcher.
        self.flashes.push(Flash {
            x,
            y,
            life: 0.18,
            max_life: 0.18,
            radius: 3.0,
            color: mix(color, [1.0, 0.7, 0.35], 0.5),
            power: 0.2,
        });
        self.total_launched += 1;
    }

    /// Launch a shell of a random style, used by the interactive "fire" key.
    pub fn fire_one(&mut self) -> Kind {
        let kind = self.pick_kind();
        let shell = self.shell(kind);
        self.launch(shell);
        kind
    }

    pub fn update(&mut self, dt: f32) {
        let launch = self
            .message
            .as_mut()
            .and_then(|m| m.tick(dt, self.w, self.h));
        if let Some(letter) = launch {
            self.launch_letter(letter);
        }
        for g in &mut self.glyphs {
            g.update(dt);
        }
        self.glyphs.retain(|g| g.alive());

        self.next_launch -= dt;
        if self.next_launch <= 0.0 {
            self.schedule();
        }

        for q in &mut self.queue {
            q.0 -= dt;
        }
        let due: Vec<Shell> = self
            .queue
            .iter()
            .filter(|q| q.0 <= 0.0)
            .map(|q| q.1)
            .collect();
        self.queue.retain(|q| q.0 > 0.0);
        for shell in due {
            self.launch(shell);
        }

        // Rockets climb; a short comet tail is drawn from their velocity.
        let mut bursts: Vec<(f32, f32, Shell)> = Vec::new();
        for r in &mut self.rockets {
            r.vy += GRAV * dt;
            r.wobble_phase += 2.6 * dt;
            r.vx += r.wobble_phase.sin() * r.wobble * dt;
            r.x += r.vx * dt;
            r.y += r.vy * dt;
            r.life -= dt;
            if r.vy >= -3.0 || r.y <= r.burst_y || r.life <= 0.0 {
                bursts.push((r.x, r.y, r.shell));
            }
        }
        self.rockets
            .retain(|r| r.life > 0.0 && r.vy < -3.0 && r.y > r.burst_y);

        for (x, y, shell) in bursts {
            self.burst(x, y, shell);
        }

        // Sparks.
        let mut spawned: Vec<Particle> = Vec::new();
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.px = p.x;
            p.py = p.y;
            p.vy += p.gravity * dt;
            let damp = (1.0 - p.drag * dt).max(0.0);
            p.vx *= damp;
            p.vy *= damp;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
            p.flicker_phase += p.flicker_rate * dt;

            if p.glitter > 0.0 && p.life > 0.0 && self.rng.random::<f32>() < p.glitter * dt {
                let a = self.rng.random::<f32>() * TAU;
                let sp = self.rng.random_range(1.0..6.0);
                let mut g = Particle::new(
                    p.x,
                    p.y,
                    p.vx * 0.3 + a.cos() * sp,
                    p.vy * 0.3 + a.sin() * sp,
                    mix(p.color, WHITE, 0.55),
                    0.12 + self.rng.random::<f32>() * 0.3,
                );
                g.gravity = 18.0;
                g.drag = 1.5;
                g.bright = 0.7;
                g.cooling = false;
                spawned.push(g);
            }

            if p.split > 0 && !p.split_done && p.life < p.max_life * p.split_at {
                p.split_done = true;
                let base = p.vx.atan2(-p.vy);
                let n = p.split;
                for k in 0..n {
                    let a = base + k as f32 * TAU / n as f32;
                    let (dx, dy) = (a.sin(), -a.cos());
                    let mut c = Particle::new(
                        p.x,
                        p.y,
                        p.vx * 0.3 + dx * p.split_speed,
                        p.vy * 0.3 + dy * p.split_speed,
                        p.split_color,
                        0.7 + self.rng.random::<f32>() * 0.35,
                    );
                    c.gravity = p.gravity * 0.9;
                    c.drag = 0.75;
                    c.bright = 1.05;
                    c.glitter = 1.5;
                    spawned.push(c);
                }
            }
        }

        let (w, h) = (self.w, self.h);
        self.particles
            .retain(|p| p.life > 0.0 && p.x > -40.0 && p.x < w + 40.0 && p.y < h + 40.0);
        self.particles.append(&mut spawned);
        if self.particles.len() > 7000 {
            let drop = self.particles.len() - 7000;
            self.particles.drain(0..drop);
        }

        for f in &mut self.flashes {
            f.life -= dt;
        }
        self.flashes.retain(|f| f.life > 0.0);
    }

    fn burst(&mut self, x: f32, y: f32, shell: Shell) {
        let kind = shell.kind;
        let s = shell.speed;
        let cs = self.count_scale;
        let mut particles: Vec<Particle> = Vec::new();

        match kind {
            Kind::Peony | Kind::Chrysanthemum => {
                let n = (280.0 * cs) as usize;
                let chry = kind == Kind::Chrysanthemum;
                for i in 0..n {
                    let (dx, dy) = sphere_dir(&mut self.rng);
                    let fill = self.rng.random::<f32>().sqrt();
                    let sp = s * (0.35 + 0.65 * fill) * self.rng.random_range(0.96..1.04);
                    let color = self.shell_color(&shell);
                    let mut p = Particle::new(
                        x,
                        y,
                        dx * sp,
                        dy * sp,
                        color,
                        1.05 + self.rng.random::<f32>() * (if chry { 0.75 } else { 0.5 }),
                    );
                    p.drag = 0.95;
                    p.gravity = 18.0;
                    p.bright = 1.0;
                    if chry {
                        p.glitter = 4.5;
                        p.flicker_amt = 0.1;
                        p.flicker_rate = 8.0;
                    }
                    // Fireworks are born white-hot; keep a few pure for sparkle.
                    if i % 23 == 0 {
                        p.color = WHITE;
                    }
                    particles.push(p);
                }
                // A few shells carry a contrasting pistil in the middle.
                if self.rng.random_bool(0.45) {
                    let core = mix(shell.b, WHITE, 0.35);
                    let cn = (40.0 * cs) as usize;
                    for _ in 0..cn {
                        let (dx, dy) = sphere_dir(&mut self.rng);
                        let sp = s * 0.24 * self.rng.random_range(0.7..1.0);
                        let mut p = Particle::new(x, y, dx * sp, dy * sp, core, 0.9);
                        p.drag = 1.3;
                        p.gravity = GRAV * 0.6;
                        p.bright = 0.8;
                        particles.push(p);
                    }
                }
            }
            Kind::Willow => {
                let n = (85.0 * cs) as usize;
                for _ in 0..n {
                    let (dx, dy) = sphere_dir(&mut self.rng);
                    let sp = s * (1.05 + self.rng.random::<f32>() * 0.45);
                    let color = mix(shell.a, [1.0, 0.84, 0.38], 0.45);
                    let mut p = Particle::new(
                        x,
                        y,
                        dx * sp,
                        dy * sp * 0.75,
                        color,
                        2.2 + self.rng.random::<f32>() * 0.8,
                    );
                    p.gravity = 22.0;
                    p.drag = 0.5;
                    p.bright = 1.1;
                    p.glitter = 2.5;
                    p.cooling = true;
                    particles.push(p);
                }
            }
            Kind::Ring => {
                // Two concentric rings, tilted so they read as a disc in space.
                let n = (150.0 * cs) as usize;
                let tilt = self.rng.random_range(0.6..1.0);
                let rot: f32 = self.rng.random_range(-0.35..0.35);
                for i in 0..n {
                    let a = i as f32 / n as f32 * TAU;
                    let (dx, dy) = (a.cos(), a.sin() * tilt);
                    let (c, sn) = (rot.cos(), rot.sin());
                    let (dx, dy) = (dx * c - dy * sn, dx * sn + dy * c);
                    // Fast kick, heavy braking: the ring stays a crisp outline
                    // instead of painting its own interior with long trails.
                    let sp = s * 1.7 * (0.97 + self.rng.random::<f32>() * 0.06);
                    let color = self.shell_color(&shell);
                    let mut p = Particle::new(x, y, dx * sp, dy * sp, color, 1.3);
                    p.drag = 3.2;
                    p.gravity = 6.0;
                    p.bright = 0.9;
                    p.trail = false;
                    p.flicker_amt = 0.1;
                    p.flicker_rate = 7.0;
                    particles.push(p);
                }
            }
            Kind::Palm => {
                let arms = self.rng.random_range(8..13);
                for _ in 0..arms {
                    let a = self.rng.random::<f32>() * TAU;
                    let (dx, dy) = (a.cos(), a.sin());
                    let segs = (10.0 * cs) as usize + 6;
                    for k in 0..segs {
                        let f = k as f32 / segs as f32;
                        let sp = s * (0.32 + 0.68 * f) * self.rng.random_range(0.94..1.06);
                        let jitter = self.rng.random_range(-0.16..0.16);
                        let color = mix(shell.a, self.shell_color(&shell), f * 0.5);
                        let mut p = Particle::new(
                            x,
                            y,
                            (dx + jitter) * sp,
                            (dy + jitter) * sp,
                            color,
                            1.5 + f * 1.0,
                        );
                        p.gravity = 42.0;
                        p.drag = 0.36;
                        p.bright = 0.95;
                        p.glitter = 3.5;
                        particles.push(p);
                    }
                }
            }
            Kind::Crossette => {
                let n = (60.0 * cs) as usize;
                for _ in 0..n {
                    let (dx, dy) = sphere_dir(&mut self.rng);
                    let sp = s * (0.75 + self.rng.random::<f32>() * 0.25);
                    let mut p = Particle::new(x, y, dx * sp, dy * sp, shell.a, 0.72);
                    p.drag = 0.5;
                    p.gravity = 30.0;
                    p.split = 4;
                    p.split_at = self.rng.random_range(0.5..0.7);
                    p.split_speed = s * self.rng.random_range(0.3..0.45);
                    p.split_color = mix(shell.b, WHITE, 0.2);
                    p.split_done = false;
                    p.bright = 1.05;
                    particles.push(p);
                }
            }
            Kind::Strobe => {
                let n = (160.0 * cs) as usize;
                for _ in 0..n {
                    let (dx, dy) = sphere_dir(&mut self.rng);
                    let sp = s * (0.8 + self.rng.random::<f32>() * 0.2);
                    let color = mix(self.shell_color(&shell), WHITE, 0.35);
                    let mut p = Particle::new(
                        x,
                        y,
                        dx * sp,
                        dy * sp,
                        color,
                        1.9 + self.rng.random::<f32>() * 0.8,
                    );
                    p.gravity = 14.0;
                    p.drag = 0.62;
                    p.flicker_amt = 0.9;
                    p.flicker_rate = self.rng.random_range(7.0..15.0);
                    p.flicker_phase = self.rng.random::<f32>() * TAU;
                    p.bright = 1.15;
                    particles.push(p);
                }
            }
            Kind::Heart => {
                let n = (190.0 * cs) as usize;
                for i in 0..n {
                    let a = i as f32 / n as f32 * TAU;
                    let hx = 16.0 * a.sin().powi(3);
                    let hy = -(13.0 * a.cos()
                        - 5.0 * (2.0 * a).cos()
                        - 2.0 * (3.0 * a).cos()
                        - (4.0 * a).cos());
                    let (mut dx, mut dy) = (hx / 17.0, hy / 17.0);
                    let jitter = self.rng.random_range(0.97..1.03);
                    let sp = s * 0.38 * jitter;
                    dx *= sp;
                    dy *= sp;
                    let color = mix(self.shell_color(&shell), [1.0, 0.22, 0.52], 0.6);
                    let mut p = Particle::new(x, y, dx, dy, color, 1.15);
                    p.gravity = 5.0;
                    p.drag = 0.72;
                    p.bright = 1.05;
                    particles.push(p);
                }
            }
            Kind::Spiral => {
                let n = (190.0 * cs) as usize;
                for _ in 0..n {
                    let u = self.rng.random::<f32>();
                    let a = self.rng.random::<f32>() * TAU + u * 4.6;
                    let sp = s * (0.32 + 0.68 * u);
                    let color = mix(shell.a, shell.b, u);
                    let mut p = Particle::new(x, y, a.cos() * sp, a.sin() * sp, color, 1.7);
                    p.drag = 0.52;
                    p.gravity = 26.0;
                    p.glitter = 1.6;
                    p.bright = 1.0;
                    particles.push(p);
                }
            }
            Kind::Letter(ch) => {
                self.glyphs
                    .push(Glyph::new(ch, x, y, shell.scale, shell.a, &mut self.rng));
                self.flashes.push(Flash {
                    x,
                    y,
                    life: 0.14,
                    max_life: 0.14,
                    radius: 2.0 + shell.scale,
                    color: shell.a,
                    power: 0.16,
                });
                return;
            }
        }

        self.particles.append(&mut particles);
        let hot = mix(mix(shell.a, shell.b, 0.5), WHITE, 0.35);
        self.flashes.push(Flash {
            x,
            y,
            life: 0.16,
            max_life: 0.16,
            radius: 2.0 + shell.speed * 0.03,
            color: hot,
            power: 0.26,
        });
    }

    /// Draw every spark. `gain` keeps brightness constant across frame rates.
    pub fn draw(&self, em: &mut Emissive, gain: f32) {
        for f in &self.flashes {
            let lf = (f.life / f.max_life).clamp(0.0, 1.0);
            let r = f.radius * (0.55 + 0.45 * lf);
            em.glow(f.x, f.y, r, f.color, f.power * lf * lf * gain);
            em.glow(
                f.x,
                f.y,
                f.radius * 4.0,
                f.color,
                f.power * 0.06 * lf * lf * gain,
            );
        }

        for p in &self.particles {
            let lf = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut b = p.bright * lf.powf(1.3) * 0.08;
            b *= (lf * 9.0).min(1.0); // snap out at the very end
            if p.flicker_amt > 0.0 {
                let f = 1.0 + p.flicker_amt * p.flicker_phase.sin();
                b *= f.max(0.0);
            }
            b *= gain;
            let mut col = p.color;
            // White-hot at birth, ember-red as the spark dies.
            col = mix(col, WHITE, (lf - 0.88).max(0.0) * 1.6);
            if p.cooling {
                col = mix(col, EMBER, (1.0 - lf).powf(2.8) * 0.28);
            }
            if p.trail {
                em.streak(p.px, p.py, p.x, p.y, col, b);
            } else {
                em.splat(p.x, p.y, col, b);
            }
        }

        for g in &self.glyphs {
            g.draw(em, gain);
        }

        for r in &self.rockets {
            // A short comet tail behind the rising shell.
            let tail = 0.085;
            let c = mix(r.color, [1.0, 0.86, 0.6], 0.55);
            em.streak(
                r.x - r.vx * tail,
                r.y - r.vy * tail,
                r.x,
                r.y,
                c,
                0.035 * gain,
            );
            em.glow(r.x, r.y, 1.9, mix(r.color, WHITE, 0.6), 0.016 * gain);
            em.splat(r.x, r.y, [1.0, 0.94, 0.8], 0.03 * gain);
        }
    }

    /// Change the palette, recolouring the message if one is being shown.
    pub fn set_palette(&mut self, palette: Option<usize>) {
        self.forced_palette = palette;
        if let Some(msg) = self.message.as_mut() {
            let idx = palette.unwrap_or_else(|| self.rng.random_range(0..PALETTES.len()));
            msg.recolor(idx, &mut self.rng);
        }
    }

    pub fn shell_count(&self) -> usize {
        self.rockets.len() + self.queue.len()
    }
}

/// Uniformly distributed direction on the unit disc, edge-weighted like a
/// projected sphere - which is exactly what a spherical shell looks like.
fn sphere_dir(rng: &mut StdRng) -> (f32, f32) {
    let z = rng.random_range(-1.0f32..1.0f32);
    let phi = rng.random::<f32>() * TAU;
    let r = (1.0 - z * z).max(0.0).sqrt();
    (r * phi.cos(), r * phi.sin())
}
