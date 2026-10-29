use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use fireworks::app::App;
use fireworks::fireworks::Kind;
use fireworks::input::Input;
use fireworks::palette::{find_palette, palette_names};
use fireworks::snapshot;

/// A pyrotechnic screensaver for the terminal.
#[derive(Parser, Debug)]
#[command(
    name = "fireworks",
    version,
    about = "A pyrotechnic screensaver for your terminal",
    after_help = "Keys: q/esc quit · space pause · f fire · c palette · w water · d stats · h help"
)]
struct Args {
    /// Target frame rate (default: 60 locally, 30 over ssh)
    #[arg(long, value_name = "FPS", value_parser = clap::value_parser!(u32).range(15..=240))]
    fps: Option<u32>,

    /// Launch rate multiplier
    #[arg(long, default_value_t = 1.0)]
    density: f32,

    /// Colour palette (see --palettes)
    #[arg(long, value_name = "NAME", default_value = "random")]
    palette: String,

    /// Seed the show for a reproducible display
    #[arg(long)]
    seed: Option<u64>,

    /// Spell a message with letter shells, one word at a time
    #[arg(long, value_name = "TEXT")]
    text: Option<String>,

    /// Exit as soon as any key is pressed
    #[arg(long)]
    any_key: bool,

    /// List the available palettes and exit
    #[arg(long)]
    palettes: bool,

    /// Render one frame headlessly to a PPM file and exit (dev aid)
    #[arg(long, value_name = "FILE", hide = true)]
    screenshot: Option<PathBuf>,

    /// Seconds to simulate before --screenshot
    #[arg(long, default_value_t = 6.0, hide = true)]
    warmup: f32,

    /// Terminal size used for --screenshot, as WxH cells
    #[arg(long, default_value = "160x48", hide = true)]
    size: String,

    /// Show the help and stats panels in --screenshot
    #[arg(long, hide = true)]
    ui: bool,

    /// Force every shell to be one style (dev aid)
    #[arg(long, value_name = "STYLE", hide = true)]
    kind: Option<String>,
}

fn main() {
    let args = Args::parse();

    if args.palettes {
        println!("{}", palette_names());
        return;
    }

    let palette = match find_palette(&args.palette) {
        Some(i) => Some(i),
        None if args.palette.eq_ignore_ascii_case("random") => None,
        None => {
            eprintln!(
                "unknown palette `{}` — try one of: {}",
                args.palette,
                palette_names()
            );
            std::process::exit(2);
        }
    };

    let seed = args.seed.unwrap_or_else(default_seed);

    let kind = match &args.kind {
        Some(name) => match Kind::from_name(name) {
            Some(k) => Some(k),
            None => {
                eprintln!("unknown style `{name}`");
                std::process::exit(2);
            }
        },
        None => None,
    };

    if let Some(path) = &args.screenshot {
        match screenshot_mode(&args, path, seed, palette, kind) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(e) => {
                eprintln!("screenshot failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Err(e) = run(args, seed, palette, kind) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// A full-screen truecolor repaint is a lot of bytes for a slow link, so a
/// remote session defaults to 30 fps unless the user overrides it.
fn remote_session() -> bool {
    std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some()
}

fn default_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
        ^ (std::process::id() as u64) << 32
}

fn run(args: Args, seed: u64, palette: Option<usize>, kind: Option<Kind>) -> io::Result<()> {
    let mut app = App::new(
        seed,
        args.density.max(0.05),
        palette,
        args.any_key,
        kind,
        args.text.clone(),
    );

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        Hide,
        SetTitle("fireworks".to_string())
    )?;

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
        hook(info);
    }));

    let mut input = Input::new()?;

    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let fps = args
        .fps
        .unwrap_or_else(|| if remote_session() { 30 } else { 60 });
    let frame = Duration::from_secs_f32(1.0 / fps as f32);
    let mut last = Instant::now();
    let result: io::Result<()> = loop {
        let start = Instant::now();
        let dt = (start - last).as_secs_f32().min(0.08);
        last = start;

        // Drain input without ever blocking: a half-received escape sequence
        // (common over ssh) must not stall the animation.
        for press in input.poll() {
            app.on_key(press);
            if app.quit {
                break;
            }
        }
        if app.quit {
            break Ok(());
        }

        app.update(dt);
        if let Err(e) = term.draw(|f| app.render(f.buffer_mut())) {
            break Err(e);
        }

        let spent = start.elapsed();
        if spent < frame {
            std::thread::sleep(frame - spent);
        }
    };

    terminal::disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, Show)?;
    term.show_cursor()?;
    result
}

fn parse_size(s: &str) -> Option<(u16, u16)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    let w: u16 = w.trim().parse().ok()?;
    let h: u16 = h.trim().parse().ok()?;
    if w == 0 || h == 0 { None } else { Some((w, h)) }
}

fn screenshot_mode(
    args: &Args,
    path: &std::path::Path,
    seed: u64,
    palette: Option<usize>,
    kind: Option<Kind>,
) -> Result<(), String> {
    let (w, h) =
        parse_size(&args.size).ok_or_else(|| "invalid --size, expected WxH".to_string())?;
    let mut app = App::new(
        seed,
        args.density.max(0.05),
        palette,
        args.any_key,
        kind,
        args.text.clone(),
    );
    app.ensure(w as usize, h as usize);

    let steps = (args.warmup.max(0.0) * 60.0) as usize;
    for _ in 0..steps {
        app.update(1.0 / 60.0);
    }

    if args.ui {
        app.help = true;
        app.stats = true;
        app.hint = 9.0;
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    app.render(&mut buf);
    snapshot::write_ppm(path, &buf, 8, 16).map_err(|e| e.to_string())
}
