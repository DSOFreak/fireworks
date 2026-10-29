//! Application state: owns the show, the buffers and the chrome.

use rand::SeedableRng;
use rand::rngs::StdRng;
use ratatui::buffer::Buffer;

use crate::fireworks::{Fireworks, Kind};
use crate::fx::Emissive;
use crate::input::{Key, KeyPress};
use crate::palette::PALETTES;
use crate::render::Renderer;
use crate::sky::Sky;
use crate::ui;

pub struct App {
    pub show: Fireworks,
    pub em: Emissive,
    pub sky: Sky,
    pub renderer: Renderer,
    pub paused: bool,
    pub help: bool,
    pub stats: bool,
    pub water: bool,
    pub any_key: bool,
    pub quit: bool,
    pub time: f32,
    pub hint: f32,
    pub toast: Option<(String, f32)>,
    pub fps: f32,
    pub palette_idx: Option<usize>,
    rng: StdRng,
}

impl App {
    pub fn new(
        seed: u64,
        density: f32,
        palette: Option<usize>,
        any_key: bool,
        kind: Option<Kind>,
        text: Option<String>,
    ) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let show = Fireworks::new(
            1.0,
            1.0,
            StdRng::seed_from_u64(seed ^ 0x9e37_79b9_7f4a_7c15),
            density,
            palette,
            kind,
            text.as_deref(),
        );
        Self {
            show,
            em: Emissive::new(1, 1),
            sky: Sky::new(1, 1, &mut rng),
            renderer: Renderer::new(),
            paused: false,
            help: false,
            stats: false,
            water: true,
            any_key,
            quit: false,
            time: 0.0,
            hint: 7.0,
            toast: None,
            fps: 0.0,
            palette_idx: palette,
            rng,
        }
    }

    /// Make sure every buffer matches the terminal size.
    pub fn ensure(&mut self, w: usize, h: usize) {
        let (w, h) = (w.max(2), h.max(2));
        if w == self.renderer.w && h == self.renderer.h {
            return;
        }
        self.renderer.ensure(w, h);
        self.em.resize(w, h * 2);
        self.sky.resize(w, h * 2, &mut self.rng);
        self.show.resize(w as f32, (h * 2) as f32);
    }

    pub fn update(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let inst = 1.0 / dt;
        self.fps = if self.fps <= 0.0 {
            inst
        } else {
            self.fps * 0.92 + inst * 0.08
        };
        self.time += dt;
        if self.hint > 0.0 {
            self.hint = (self.hint - dt).max(0.0);
        }
        if let Some(t) = &mut self.toast {
            t.1 -= dt;
        }
        if self.toast.as_ref().is_some_and(|t| t.1 <= 0.0) {
            self.toast = None;
        }
        if self.paused {
            return;
        }

        // Frame-rate independent brightness and trail length.
        let gain = (dt * 60.0).clamp(0.2, 2.5);
        self.em.fade(0.05f32.powf(dt));
        self.show.update(dt);
        self.sky.update(dt, self.em.w, self.em.h, &mut self.rng);
        self.show.draw(&mut self.em, gain);
        self.em.decay_clamp(8.0);
    }

    pub fn render(&mut self, buf: &mut Buffer) {
        let area = *buf.area();
        self.ensure(area.width as usize, area.height as usize);
        self.renderer
            .compose(&self.em, &self.sky, self.time, self.water);
        self.renderer.blit(buf, area);
        ui::draw(self, buf, area);
    }

    pub fn on_key(&mut self, press: KeyPress) {
        let KeyPress { key, ctrl, .. } = press;
        if ctrl && key == Key::Char('c') {
            self.quit = true;
            return;
        }
        if self.any_key && key != Key::Eof {
            self.quit = true;
            return;
        }
        match key {
            Key::Eof | Key::Esc | Key::Char('q') => self.quit = true,
            Key::Char(' ') => {
                self.paused = !self.paused;
                self.toast(if self.paused { "paused" } else { "resumed" });
            }
            Key::Char('f') => {
                let kind = self.show.fire_one();
                self.toast(format!("{} away", kind.label()));
            }
            Key::Char('h') | Key::Char('?') => self.help = !self.help,
            Key::Char('d') => self.stats = !self.stats,
            Key::Char('w') => {
                self.water = !self.water;
                self.toast(if self.water { "water on" } else { "water off" });
            }
            Key::Char('c') => self.cycle_palette(),
            Key::Char('+') | Key::Char('=') => self.bump_density(1.25),
            Key::Char('-') | Key::Char('_') => self.bump_density(0.8),
            Key::Char('s') => {
                self.sky.enabled = !self.sky.enabled;
                self.toast(if self.sky.enabled {
                    "stars on"
                } else {
                    "stars off"
                });
            }
            Key::Char('l') => {
                self.sky.shore_enabled = !self.sky.shore_enabled;
                self.toast(if self.sky.shore_enabled {
                    "skyline on"
                } else {
                    "skyline off"
                });
            }
            _ => {}
        }
    }

    fn bump_density(&mut self, factor: f32) {
        self.show.density = (self.show.density * factor).clamp(0.2, 6.0);
        self.toast(format!("launch rate ×{:.1}", self.show.density));
    }

    fn cycle_palette(&mut self) {
        self.palette_idx = match self.palette_idx {
            None => Some(0),
            Some(i) if i + 1 < PALETTES.len() => Some(i + 1),
            Some(_) => None,
        };
        self.show.set_palette(self.palette_idx);
        let name = match self.palette_idx {
            None => "random",
            Some(i) => PALETTES[i].name,
        };
        self.toast(format!("palette · {name}"));
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 1.8));
    }

    pub fn palette_name(&self) -> &'static str {
        match self.palette_idx {
            None => "random",
            Some(i) => PALETTES[i].name,
        }
    }

    pub fn spark_count(&self) -> usize {
        self.show.particles.len()
    }
}
