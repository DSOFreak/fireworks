//! The small amount of screen chrome: help, stats, hint and toasts.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};

use crate::app::App;

const GOLD: Color = Color::Rgb(255, 212, 140);
const INK: Color = Color::Rgb(148, 160, 190);
const BRIGHT: Color = Color::Rgb(226, 233, 250);
const PANEL_BG: Color = Color::Rgb(9, 12, 24);
const HINT_BG: Color = Color::Rgb(6, 8, 18);
const BORDER: Color = Color::Rgb(74, 90, 136);

pub fn draw(app: &App, buf: &mut Buffer, area: Rect) {
    if app.help {
        draw_help(app, buf, area);
    } else if app.hint > 0.0 {
        draw_hint(app, buf, area);
    }
    if let Some((msg, left)) = &app.toast {
        draw_toast(buf, area, msg, *left);
    }
    if app.stats {
        draw_stats(app, buf, area);
    }
}

fn dim(alpha: f32) -> Color {
    let a = alpha.clamp(0.0, 1.0);
    let f = |v: f32| (v * a) as u8;
    Color::Rgb(f(148.0), f(160.0), f(190.0))
}

fn draw_hint(app: &App, buf: &mut Buffer, area: Rect) {
    if area.width < 34 || area.height < 2 {
        return;
    }
    let alpha = (app.hint / 1.6).min(1.0);
    let text = " h help   ·   space pause   ·   f fire   ·   q quit ";
    let w = text.chars().count() as u16;
    if w + 2 > area.width {
        return;
    }
    let rect = Rect::new(area.width - w - 1, area.height - 1, w, 1);
    Clear.render(rect, buf);
    Paragraph::new(Line::from(text))
        .style(Style::default().fg(dim(alpha)).bg(HINT_BG))
        .render(rect, buf);
}

fn draw_toast(buf: &mut Buffer, area: Rect, msg: &str, left: f32) {
    if area.height < 3 || area.width < 12 {
        return;
    }
    let alpha = (left / 0.6).min(1.0);
    let w = msg.chars().count() as u16 + 4;
    let x = area.width.saturating_sub(w) / 2;
    let rect = Rect::new(x, area.height - 2, w, 1);
    Clear.render(rect, buf);
    let fg = Color::Rgb(
        (255.0 * alpha) as u8,
        (216.0 * alpha) as u8,
        (150.0 * alpha) as u8,
    );
    Paragraph::new(Line::from(Span::styled(
        format!("  {msg}  "),
        Style::default().fg(fg).add_modifier(Modifier::BOLD),
    )))
    .style(Style::default().bg(PANEL_BG))
    .alignment(Alignment::Center)
    .render(rect, buf);
}

fn draw_stats(app: &App, buf: &mut Buffer, area: Rect) {
    let w = 31u16;
    let h = 5u16;
    if area.width < w + 2 || area.height < h + 2 {
        return;
    }
    let rect = Rect::new(area.width - w - 1, 1, w, h);
    panel(buf, rect, "stats");
    let inner = rect.inner(ratatui::layout::Margin::new(2, 1));
    let lines = vec![
        Line::from(vec![
            Span::styled("fps ", Style::default().fg(INK)),
            Span::styled(
                format!("{:>5.1}", app.fps),
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   sparks ", Style::default().fg(INK)),
            Span::styled(
                format!("{:>4}", app.spark_count()),
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("shells ", Style::default().fg(INK)),
            Span::styled(
                format!("{:>2}", app.show.shell_count()),
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   res ", Style::default().fg(INK)),
            Span::styled(
                format!("{}x{}", app.renderer.sw, app.renderer.sh),
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("palette ", Style::default().fg(INK)),
            Span::styled(app.palette_name(), Style::default().fg(GOLD)),
            Span::styled("  rate ", Style::default().fg(INK)),
            Span::styled(
                format!("×{:.1}", app.show.density),
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    Paragraph::new(lines).render(inner, buf);
}

fn draw_help(app: &App, buf: &mut Buffer, area: Rect) {
    let w = 52u16.min(area.width.saturating_sub(2));
    let h = 15u16.min(area.height.saturating_sub(2));
    if w < 24 || h < 8 {
        return;
    }
    let rect = Rect::new(
        (area.width.saturating_sub(w)) / 2,
        (area.height.saturating_sub(h)) / 2,
        w,
        h,
    );
    panel(buf, rect, "fireworks");

    let key = |k: &'static str, desc: &'static str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {k:<8}"), Style::default().fg(GOLD)),
            Span::styled(desc, Style::default().fg(BRIGHT)),
        ])
    };
    let mut lines = vec![
        Line::from(Span::styled(
            "  a terminal pyrotechnic show",
            Style::default().fg(INK),
        )),
        Line::from(""),
        key("space", "pause / resume"),
        key("f", "launch a shell"),
        key("c", "cycle palette"),
        key("w", "water reflection"),
        key("s", "stars"),
        key("l", "skyline"),
        key("+ / -", "launch rate"),
        key("d", "performance stats"),
    ];
    lines.push(Line::from(vec![
        Span::styled("  palette  ", Style::default().fg(INK)),
        Span::styled(app.palette_name(), Style::default().fg(GOLD)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  h  ", Style::default().fg(GOLD)),
        Span::styled("close   ", Style::default().fg(INK)),
        Span::styled("q / esc  ", Style::default().fg(GOLD)),
        Span::styled("quit", Style::default().fg(INK)),
    ]));

    let inner = rect.inner(ratatui::layout::Margin::new(1, 1));
    let top = inner.y
        + if lines.len() as u16 > inner.height {
            0
        } else {
            (inner.height - lines.len() as u16) / 2
        };
    let body = Rect::new(inner.x, top, inner.width, lines.len() as u16);
    Paragraph::new(lines).render(body, buf);
}

fn panel(buf: &mut Buffer, rect: Rect, title: &str) {
    Clear.render(rect, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANEL_BG))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
        ));
    block.render(rect, buf);
}
