//! Letter shells: fireworks that spell a message out, one word at a time.
//!
//! A word is laid out as a row of glyphs. Each glyph gets its own rocket; when
//! the rocket bursts, the sparks fly out and settle onto the lit pixels of a
//! 5x7 bitmap font, so the word appears left to right as the shells go up.

use std::f32::consts::TAU;

use rand::Rng;
use rand::rngs::StdRng;

use crate::fx::Emissive;
use crate::palette::{PALETTES, Rgb, WHITE, mix};

pub const GLYPH_W: usize = 5;
pub const GLYPH_H: usize = 7;

/// Bit rows, top to bottom. `1` is a lit pixel.
const FONT: &[(char, &str)] = &[
    ('A', "01110 10001 10001 11111 10001 10001 10001"),
    ('B', "11110 10001 10001 11110 10001 10001 11110"),
    ('C', "01110 10001 10000 10000 10000 10001 01110"),
    ('D', "11110 10001 10001 10001 10001 10001 11110"),
    ('E', "11111 10000 10000 11110 10000 10000 11111"),
    ('F', "11111 10000 10000 11110 10000 10000 10000"),
    ('G', "01110 10001 10000 10111 10001 10001 01111"),
    ('H', "10001 10001 10001 11111 10001 10001 10001"),
    ('I', "01110 00100 00100 00100 00100 00100 01110"),
    ('J', "00111 00010 00010 00010 00010 10010 01100"),
    ('K', "10001 10010 10100 11000 10100 10010 10001"),
    ('L', "10000 10000 10000 10000 10000 10000 11111"),
    ('M', "10001 11011 10101 10101 10001 10001 10001"),
    ('N', "10001 11001 10101 10011 10001 10001 10001"),
    ('O', "01110 10001 10001 10001 10001 10001 01110"),
    ('P', "11110 10001 10001 11110 10000 10000 10000"),
    ('Q', "01110 10001 10001 10001 10101 10010 01101"),
    ('R', "11110 10001 10001 11110 10100 10010 10001"),
    ('S', "01111 10000 10000 01110 00001 00001 11110"),
    ('T', "11111 00100 00100 00100 00100 00100 00100"),
    ('U', "10001 10001 10001 10001 10001 10001 01110"),
    ('V', "10001 10001 10001 10001 10001 01010 00100"),
    ('W', "10001 10001 10001 10101 10101 11011 10001"),
    ('X', "10001 10001 01010 00100 01010 10001 10001"),
    ('Y', "10001 10001 01010 00100 00100 00100 00100"),
    ('Z', "11111 00001 00010 00100 01000 10000 11111"),
    ('0', "01110 10001 10011 10101 11001 10001 01110"),
    ('1', "00100 01100 00100 00100 00100 00100 01110"),
    ('2', "01110 10001 00001 00010 00100 01000 11111"),
    ('3', "11111 00010 00100 00010 00001 10001 01110"),
    ('4', "00010 00110 01010 10010 11111 00010 00010"),
    ('5', "11111 10000 11110 00001 00001 10001 01110"),
    ('6', "00110 01000 10000 11110 10001 10001 01110"),
    ('7', "11111 00001 00010 00100 01000 01000 01000"),
    ('8', "01110 10001 10001 01110 10001 10001 01110"),
    ('9', "01110 10001 10001 01111 00001 00010 01100"),
    ('!', "00100 00100 00100 00100 00100 00000 00100"),
    ('?', "01110 10001 00001 00010 00100 00000 00100"),
    ('.', "00000 00000 00000 00000 00000 00110 00110"),
    (',', "00000 00000 00000 00000 00110 00110 00100"),
    ('\'', "00100 00100 00000 00000 00000 00000 00000"),
    ('-', "00000 00000 00000 11111 00000 00000 00000"),
    ('_', "00000 00000 00000 00000 00000 00000 11111"),
    ('+', "00000 00100 00100 11111 00100 00100 00000"),
    ('=', "00000 00000 11111 00000 11111 00000 00000"),
    ('/', "00001 00010 00010 00100 01000 01000 10000"),
    (':', "00000 00110 00110 00000 00110 00110 00000"),
    ('#', "01010 01010 11111 01010 11111 01010 01010"),
    ('*', "00000 10101 01110 11111 01110 10101 00000"),
    ('&', "01100 10010 10100 01000 10101 10010 01101"),
    ('<', "00010 00100 01000 10000 01000 00100 00010"),
    ('>', "01000 00100 00010 00001 00010 00100 01000"),
    ('(', "00010 00100 01000 01000 01000 00100 00010"),
    (')', "01000 00100 00010 00010 00010 00100 01000"),
    ('%', "10001 00010 00100 00100 01000 10001 00000"),
    ('♥', "00000 01010 11111 11111 01110 00100 00000"),
    (' ', "00000 00000 00000 00000 00000 00000 00000"),
];

const FALLBACK: &str = "01110 10001 00001 00010 00100 00000 00100";

/// Bitmap rows for `ch`, falling back to `?`.
pub fn bitmap(ch: char) -> [u8; GLYPH_H] {
    let ch = ch.to_ascii_uppercase();
    let rows = FONT
        .iter()
        .find(|(c, _)| *c == ch)
        .map(|(_, art)| *art)
        .unwrap_or(FALLBACK);
    let mut out = [0u8; GLYPH_H];
    for (i, word) in rows.split_whitespace().take(GLYPH_H).enumerate() {
        out[i] = u8::from_str_radix(word, 2).unwrap_or(0);
    }
    out
}

/// Lit pixel coordinates of a glyph, as (column, row) from the top left.
pub fn dots(ch: char) -> Vec<(usize, usize)> {
    let rows = bitmap(ch);
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for x in 0..GLYPH_W {
            if row & (1 << (GLYPH_W - 1 - x)) != 0 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Where one letter of a salvo goes.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

/// Glyph size that lets `len` letters share the line.
pub fn scale_for(len: usize, w: f32) -> f32 {
    let n = len.max(1) as f32;
    (w * 0.74 / (n * 6.0 - 1.0).max(1.0)).clamp(0.9, 4.4)
}

/// Lay `len` letters out on one line at `scale`, centred horizontally.
pub fn slots(len: usize, scale: f32, w: f32, h: f32) -> Vec<Slot> {
    let n = len.max(1) as f32;
    let total = (n * 6.0 - 1.0) * scale;
    let x0 = w * 0.5 - total * 0.5 + 2.5 * scale;
    let y = h * 0.34;
    (0..len)
        .map(|i| Slot {
            x: x0 + i as f32 * 6.0 * scale,
            y,
            scale,
        })
        .collect()
}

/// How many letters fit on one line without turning them into mush.
pub fn letters_per_line(w: f32) -> usize {
    ((w * 0.74 / 6.0 / 1.9) as usize).max(4)
}

struct Spark {
    sx: f32,
    sy: f32,
    tx: f32,
    ty: f32,
    x: f32,
    y: f32,
    px: f32,
    py: f32,
    delay: f32,
    phase: f32,
}

/// One letter, forming out of a shell burst.
pub struct Glyph {
    pub ch: char,
    color: Rgb,
    sparks: Vec<Spark>,
    age: f32,
    form: f32,
    hold: f32,
    fade: f32,
}

impl Glyph {
    pub fn new(ch: char, cx: f32, cy: f32, scale: f32, color: Rgb, rng: &mut StdRng) -> Self {
        let mut sparks = Vec::new();
        for (col, row) in dots(ch) {
            let tx = cx + (col as f32 + 0.5 - GLYPH_W as f32 * 0.5) * scale;
            let ty = cy + (row as f32 + 0.5 - GLYPH_H as f32 * 0.5) * scale;
            // Denser sparks for bigger glyphs keep the strokes solid.
            let per_dot = (scale * 1.2).clamp(2.0, 4.0) as usize;
            for k in 0..per_dot {
                let a = rng.random::<f32>() * TAU;
                let r = rng.random_range(1.5..7.0) * (1.0 + k as f32 * 0.15);
                let sx = cx + a.cos() * r;
                let sy = cy + a.sin() * r * 0.7;
                sparks.push(Spark {
                    sx,
                    sy,
                    tx: tx + rng.random_range(-0.3..0.3) * scale,
                    ty: ty + rng.random_range(-0.3..0.3) * scale,
                    x: sx,
                    y: sy,
                    px: sx,
                    py: sy,
                    delay: k as f32 * 0.035 + rng.random::<f32>() * 0.08,
                    phase: rng.random::<f32>() * TAU,
                });
            }
        }
        Self {
            ch,
            color,
            sparks,
            age: 0.0,
            form: 0.65,
            hold: 2.4,
            fade: 1.4,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        for s in &mut self.sparks {
            s.px = s.x;
            s.py = s.y;
            let t = ((self.age - s.delay) / self.form).clamp(0.0, 1.0);
            let ease = 1.0 - (1.0 - t).powi(3);
            s.x = s.sx + (s.tx - s.sx) * ease;
            s.y = s.sy + (s.ty - s.sy) * ease;
        }
    }

    pub fn alive(&self) -> bool {
        self.age < self.form + self.hold + self.fade
    }

    pub fn draw(&self, em: &mut Emissive, gain: f32) {
        let settled = self.form + 0.15;
        let fade = if self.age <= settled + self.hold {
            1.0
        } else {
            (1.0 - (self.age - settled - self.hold) / self.fade).clamp(0.0, 1.0)
        };
        for s in &self.sparks {
            let t = ((self.age - s.delay) / self.form).clamp(0.0, 1.0);
            // Sparks are dim while they fly and brighten as they take position.
            let twinkle = 0.88 + 0.12 * (self.age * 5.0 + s.phase).sin();
            let b = 0.075 * gain * fade * twinkle * (0.35 + 0.65 * t);
            let white = (1.0 - t) * 0.3 + if t > 0.9 { 0.12 } else { 0.0 };
            em.streak(s.px, s.py, s.x, s.y, mix(self.color, WHITE, white), b);
        }
    }
}

/// A salvo to launch: one letter at its place in the word.
#[derive(Clone, Copy, Debug)]
pub struct Launch {
    pub ch: char,
    pub color: Rgb,
    pub slot: Slot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Gap,
    Show,
    Hold,
}

/// Walks the message one line at a time, forever.
pub struct Message {
    words: Vec<Vec<char>>,
    colors: Vec<Vec<Rgb>>,
    word: usize,
    next_letter: usize,
    line_len: usize,
    line_pos: usize,
    scale: f32,
    phase: Phase,
    timer: f32,
}

impl Message {
    pub fn new(text: &str, palette: usize, rng: &mut StdRng) -> Option<Self> {
        let words: Vec<Vec<char>> = text
            .split_whitespace()
            .map(|w| {
                w.chars()
                    .map(|c| c.to_ascii_uppercase())
                    .collect::<Vec<_>>()
            })
            .filter(|w: &Vec<char>| !w.is_empty())
            .collect();
        if words.is_empty() {
            return None;
        }
        let mut msg = Self {
            words,
            colors: Vec::new(),
            word: 0,
            next_letter: 0,
            line_len: 0,
            line_pos: 0,
            scale: 1.0,
            phase: Phase::Gap,
            timer: 1.2,
        };
        msg.recolor(palette, rng);
        Some(msg)
    }

    /// Re-sample the letter colours, e.g. when the palette is cycled at runtime.
    pub fn recolor(&mut self, palette: usize, rng: &mut StdRng) {
        let Some(stops) = PALETTES.get(palette) else {
            return;
        };
        self.colors = self
            .words
            .iter()
            .map(|word| {
                let n = word.len() as f32;
                word.iter()
                    .enumerate()
                    .map(|(i, _)| {
                        // Spread the palette across the word so it reads in colour.
                        let t = (i as f32 + 0.4) / n + rng.random_range(0.0..0.05);
                        stops.at(t)
                    })
                    .collect()
            })
            .collect();
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// Advance the schedule and return the next letter to launch, if any.
    pub fn tick(&mut self, dt: f32, w: f32, h: f32) -> Option<Launch> {
        self.timer -= dt;
        if self.timer > 0.0 {
            return None;
        }
        match self.phase {
            Phase::Gap => {
                self.phase = Phase::Show;
                self.next_letter = 0;
                self.line_len = 0;
                self.timer = 0.0;
                None
            }
            Phase::Show => {
                let word_len = self.words[self.word].len();
                if self.line_len == 0 {
                    if self.next_letter >= word_len {
                        self.phase = Phase::Hold;
                        self.timer = 4.2;
                        return None;
                    }
                    let per_line = letters_per_line(w);
                    // Every line of a wrapped word keeps the same letter size.
                    self.scale = scale_for(word_len.min(per_line), w);
                    self.line_len = (word_len - self.next_letter).min(per_line);
                    self.line_pos = 0;
                }
                let len = self.line_len;
                let slots = slots(len, self.scale, w, h);
                let ch = self.words[self.word][self.next_letter];
                let color = self.colors[self.word][self.next_letter];
                let slot = slots[self.line_pos];
                self.line_pos += 1;
                self.next_letter += 1;
                if self.line_pos == len {
                    // End of a line: let it settle before the next one.
                    self.phase = Phase::Hold;
                    self.line_len = 0;
                    self.timer = if self.next_letter < word_len {
                        3.2
                    } else {
                        4.4
                    };
                } else {
                    self.timer = 0.13;
                }
                Some(Launch { ch, color, slot })
            }
            Phase::Hold => {
                if self.next_letter < self.words[self.word].len() {
                    self.phase = Phase::Show;
                    self.timer = 0.25;
                } else {
                    self.word = (self.word + 1) % self.words.len();
                    self.next_letter = 0;
                    self.phase = Phase::Gap;
                    self.timer = if self.word == 0 { 2.6 } else { 1.2 };
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn every_glyph_is_well_formed() {
        for (ch, art) in FONT {
            let rows: Vec<&str> = art.split_whitespace().collect();
            assert_eq!(rows.len(), GLYPH_H, "glyph {ch} needs {GLYPH_H} rows");
            for row in rows {
                assert_eq!(row.len(), GLYPH_W, "glyph {ch} row width");
                assert!(
                    row.chars().all(|c| c == '0' || c == '1'),
                    "glyph {ch} must be binary art"
                );
            }
        }
    }

    #[test]
    fn letters_and_digits_have_pixels() {
        for ch in ('A'..='Z').chain('0'..='9') {
            assert!(!dots(ch).is_empty(), "glyph {ch} is empty");
        }
        for ch in ['!', '?', '.', '-', '♥'] {
            assert!(!dots(ch).is_empty(), "glyph {ch} is empty");
        }
    }

    #[test]
    fn lowercase_and_unknown_characters_fall_back_sanely() {
        assert_eq!(bitmap('a'), bitmap('A'));
        assert_eq!(bitmap('€'), bitmap('?'));
        assert!(!dots('€').is_empty());
    }

    #[test]
    fn layout_fits_the_line_and_is_centred() {
        for len in 1..=12 {
            let scale = scale_for(len, 160.0);
            let slots = slots(len, scale, 160.0, 96.0);
            let half = 2.5 * scale;
            assert!(slots[0].x - half >= -0.5, "len {len} overflows left");
            assert!(
                slots[len - 1].x + half <= 160.5,
                "len {len} overflows right"
            );
            let mid = (slots[0].x + slots[len - 1].x) * 0.5;
            assert!((mid - 80.0).abs() < 0.5, "len {len} is not centred");
            // Letters never overlap.
            if len > 1 {
                assert!(slots[1].x - slots[0].x >= 5.0 * scale - 0.01);
            }
        }
    }

    #[test]
    fn glyphs_settle_onto_their_targets() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut g = Glyph::new('A', 100.0, 50.0, 2.5, [1.0, 1.0, 1.0], &mut rng);
        for _ in 0..120 {
            g.update(1.0 / 60.0);
        }
        for s in &g.sparks {
            assert!((s.x - s.tx).abs() < 0.75, "x {} vs {}", s.x, s.tx);
            assert!((s.y - s.ty).abs() < 0.75, "y {} vs {}", s.y, s.ty);
        }
    }

    #[test]
    fn message_launches_every_letter_once_and_loops() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut msg = Message::new("AB CD", 0, &mut rng).unwrap();
        assert_eq!(msg.word_count(), 2);
        let mut seen = Vec::new();
        for _ in 0..60 * 21 {
            if let Some(l) = msg.tick(1.0 / 60.0, 160.0, 96.0) {
                seen.push(l.ch);
            }
        }
        assert!(seen.len() >= 8, "expected a loop, got {seen:?}");
        assert_eq!(&seen[..4], &['A', 'B', 'C', 'D']);
        assert_eq!(&seen[4..8], &['A', 'B', 'C', 'D']);
    }

    #[test]
    fn long_words_are_split_into_lines_without_losing_letters() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut msg = Message::new("CONGRATULATIONS", 0, &mut rng).unwrap();
        let mut seen = Vec::new();
        for _ in 0..60 * 8 {
            if let Some(l) = msg.tick(1.0 / 60.0, 160.0, 96.0) {
                seen.push(l.ch);
            }
        }
        let expected: Vec<char> = "CONGRATULATIONS".chars().collect();
        assert_eq!(&seen[..expected.len()], &expected[..]);
    }

    #[test]
    fn an_empty_message_is_rejected() {
        let mut rng = StdRng::seed_from_u64(3);
        assert!(Message::new("   \n\t ", 0, &mut rng).is_none());
    }
}
