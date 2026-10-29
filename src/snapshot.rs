//! Dev helper: rasterise a rendered frame to a PPM so the look can be checked
//! without a terminal.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::style::Color;

fn rgb(c: Color) -> [u8; 3] {
    match c {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Reset => [0, 0, 0],
        other => {
            // Snapshot only ever sees RGB cells; map anything else to grey.
            let _ = other;
            [128, 128, 128]
        }
    }
}

/// Write `buf` as a binary PPM, `cw` x `ch` pixels per cell.
pub fn write_ppm(path: &Path, buf: &Buffer, cw: u32, ch: u32) -> std::io::Result<()> {
    let area = buf.area();
    let iw = area.width as u32 * cw;
    let ih = area.height as u32 * ch;
    let mut out = BufWriter::new(File::create(path)?);
    write!(out, "P6\n{iw} {ih}\n255\n")?;
    let mut px = vec![0u8; (iw * ih * 3) as usize];

    for cy in 0..area.height {
        for cx in 0..area.width {
            let Some(cell) = buf.cell((area.x + cx, area.y + cy)) else {
                continue;
            };
            let fg = rgb(cell.fg);
            let bg = rgb(cell.bg);
            let sym = cell.symbol();
            let mut glyph = vec![vec![None::<[u8; 3]>; cw as usize]; ch as usize];

            match sym {
                "▀" => {
                    for (y, row) in glyph.iter_mut().enumerate() {
                        for p in row.iter_mut() {
                            *p = Some(if y < ch as usize / 2 { fg } else { bg });
                        }
                    }
                }
                " " | "" => {
                    for row in glyph.iter_mut() {
                        for p in row.iter_mut() {
                            *p = Some(bg);
                        }
                    }
                }
                "█" => {
                    for row in glyph.iter_mut() {
                        for p in row.iter_mut() {
                            *p = Some(fg);
                        }
                    }
                }
                _ => {
                    for row in glyph.iter_mut() {
                        for p in row.iter_mut() {
                            *p = Some(bg);
                        }
                    }
                    // Rough glyph: border chars become lines, text becomes bars.
                    let top = matches!(sym, "╭" | "╮" | "─");
                    let bottom = matches!(sym, "╰" | "╯" | "─");
                    let left = matches!(sym, "╭" | "╰" | "│");
                    let right = matches!(sym, "╮" | "╯" | "│");
                    if top || bottom || left || right {
                        for (y, row) in glyph.iter_mut().enumerate() {
                            for (x, p) in row.iter_mut().enumerate() {
                                let yt = y == 0 || y == 1;
                                let yb = y + 2 >= ch as usize;
                                let xl = x == 0;
                                let xr = x + 1 >= cw as usize;
                                if (top && yt) || (bottom && yb) || (left && xl) || (right && xr) {
                                    *p = Some(fg);
                                }
                            }
                        }
                    } else if !sym.trim().is_empty() {
                        let y0 = ch as usize * 3 / 10;
                        let y1 = (ch as usize * 7) / 10;
                        let x0 = cw as usize / 8;
                        let x1 = cw as usize - cw as usize / 8;
                        for (y, row) in glyph.iter_mut().enumerate() {
                            for (x, p) in row.iter_mut().enumerate() {
                                if y >= y0 && y < y1 && x >= x0 && x < x1 {
                                    *p = Some(fg);
                                }
                            }
                        }
                    }
                }
            }

            for (y, row) in glyph.iter().enumerate() {
                for (x, p) in row.iter().enumerate() {
                    let c = p.unwrap_or(bg);
                    let ox = cx as u32 * cw + x as u32;
                    let oy = cy as u32 * ch + y as u32;
                    let i = ((oy * iw + ox) * 3) as usize;
                    px[i] = c[0];
                    px[i + 1] = c[1];
                    px[i + 2] = c[2];
                }
            }
        }
    }

    out.write_all(&px)?;
    out.flush()
}
