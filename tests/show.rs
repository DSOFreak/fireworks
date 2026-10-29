use fireworks::app::App;
use fireworks::input::{Key, KeyPress};
use fireworks::palette::{PALETTES, find_palette, mix, palette_names};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

fn show(seed: u64, w: u16, h: u16, frames: usize) -> App {
    let mut app = App::new(seed, 1.5, None, false, None, None);
    app.ensure(w as usize, h as usize);
    for _ in 0..frames {
        app.update(1.0 / 60.0);
    }
    app
}

#[test]
fn palettes_wrap_in_both_directions() {
    for p in &PALETTES {
        for t in [-3.2, -1.0, 0.0, 0.25, 0.999, 1.0, 7.5] {
            let c = p.at(t);
            for ch in c {
                assert!(ch.is_finite(), "palette {} at {t} is not finite", p.name);
                assert!((0.0..=1.0).contains(&ch));
            }
        }
        assert_eq!(p.at(0.0), p.at(1.0));
    }
}

#[test]
fn palette_lookup_is_case_insensitive() {
    assert_eq!(find_palette("RAINBOW"), Some(PALETTES.len() - 1));
    assert_eq!(find_palette("random"), None);
    assert_eq!(find_palette("nope"), None);
    assert!(palette_names().starts_with("random, "));
}

#[test]
fn mix_clamps_its_input() {
    let a = [0.0, 0.0, 0.0];
    let b = [1.0, 1.0, 1.0];
    assert_eq!(mix(a, b, -1.0), a);
    assert_eq!(mix(a, b, 2.0), b);
    assert_eq!(mix(a, b, 0.5), [0.5, 0.5, 0.5]);
}

#[test]
fn the_show_runs_without_panicking_at_odd_sizes() {
    for (w, h) in [(1u16, 1u16), (2, 2), (7, 3), (80, 24), (160, 48)] {
        let mut app = show(9, w, h, 240);
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        app.render(&mut buf);
        // Every cell is painted, and all but the chrome is half blocks.
        let mut blocks = 0;
        for y in 0..h {
            for x in 0..w {
                let cell = buf.cell((x, y)).expect("cell in range");
                if cell.symbol() == "▀" {
                    blocks += 1;
                }
                assert!(
                    matches!(cell.fg, Color::Rgb(..)),
                    "unpainted fg at ({x},{y}) of {w}x{h}"
                );
                assert!(
                    matches!(cell.bg, Color::Rgb(..)),
                    "unpainted bg at ({x},{y}) of {w}x{h}"
                );
            }
        }
        assert!(blocks >= (w as usize * h as usize) * 4 / 5);
    }
}

#[test]
fn sparks_stay_inside_the_arena() {
    let app = show(3, 120, 36, 90);
    for p in &app.show.particles {
        assert!(p.x.is_finite() && p.y.is_finite(), "non-finite particle");
        assert!(p.vx.is_finite() && p.vy.is_finite());
        assert!(p.x > -60.0 && p.x < 120.0 + 60.0);
        assert!(p.y > -60.0 && p.y < 72.0 + 60.0);
    }
    assert!(app.show.particles.len() < 7000);
}

#[test]
fn rockets_always_burst() {
    // Every rocket must turn into sparks: none may live forever as a dud.
    let mut app = show(5, 100, 30, 60);
    let mut seen_bursts = 0;
    for _ in 0..60 * 20 {
        app.update(1.0 / 60.0);
        if app.show.total_launched > seen_bursts {
            seen_bursts = app.show.total_launched;
        }
        assert!(app.show.rockets.len() < 40, "rockets piling up");
    }
    assert!(seen_bursts > 5, "expected a show, got {seen_bursts} shells");
}

#[test]
fn the_same_seed_gives_the_same_show() {
    let a = show(77, 90, 28, 300);
    let b = show(77, 90, 28, 300);
    assert_eq!(a.show.total_launched, b.show.total_launched);
    assert_eq!(a.show.particles.len(), b.show.particles.len());
    assert_eq!(a.time, b.time);

    let c = show(78, 90, 28, 300);
    assert_ne!(a.show.total_launched, c.show.total_launched);
}

#[test]
fn pause_freezes_the_simulation() {
    let mut app = show(11, 80, 24, 120);
    app.on_key(KeyPress::plain(Key::Char(' ')));
    let before = app.show.particles.len();
    let time = app.time;
    for _ in 0..120 {
        app.update(1.0 / 60.0);
    }
    assert_eq!(app.show.particles.len(), before);
    assert!(app.time > time, "clock should still run");
    assert!(!app.quit);
}

#[test]
fn any_key_mode_quits_on_a_letter() {
    let mut app = App::new(1, 1.0, None, true, None, None);
    app.on_key(KeyPress::plain(Key::Char('x')));
    assert!(app.quit);
}

#[test]
fn resize_keeps_the_show_alive() {
    let mut app = show(4, 60, 20, 90);
    let sparks = app.show.particles.len();
    app.ensure(140, 44);
    assert!(sparks > 0);
    app.update(1.0 / 60.0);
    for p in &app.show.particles {
        assert!(p.x.is_finite());
    }
}

#[test]
fn a_message_spells_itself_out() {
    let mut app = App::new(3, 1.0, None, false, None, Some("HI".to_string()));
    app.ensure(120, 36);
    for _ in 0..60 * 5 {
        app.update(1.0 / 60.0);
    }
    assert!(
        !app.show.glyphs.is_empty(),
        "the letter shells never formed a glyph"
    );
    // Text mode keeps the sky clear: only the letters are fired.
    assert!(
        app.show.total_launched <= 3,
        "random shells leaked into text mode"
    );
    let mut buf = Buffer::empty(Rect::new(0, 0, 120, 36));
    app.render(&mut buf);
}

#[test]
fn text_mode_still_quits_and_renders_at_tiny_sizes() {
    let mut app = App::new(4, 1.0, None, false, None, Some("HELLO".to_string()));
    for (w, h) in [(1u16, 1u16), (12, 6), (40, 12)] {
        app.ensure(w as usize, h as usize);
        for _ in 0..120 {
            app.update(1.0 / 60.0);
        }
        let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
        app.render(&mut buf);
    }
    app.on_key(KeyPress::plain(Key::Char('q')));
    assert!(app.quit);
}
