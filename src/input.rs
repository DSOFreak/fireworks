//! Non-blocking keyboard input.
//!
//! Crossterm's event reader is synchronous and reads the tty with blocking
//! `read(2)` calls. When a terminal delivers an escape sequence in pieces -
//! routine over SSH, where TCP happily splits a write - the parser waits for
//! the rest of the sequence on a blocking read and the whole frame loop stops
//! until the next keystroke arrives.
//!
//! This module polls the tty with a zero timeout, reads only what is already
//! there, and keeps a partial escape sequence buffered for the next frame. A
//! sequence that never completes is discarded after a few milliseconds, so a
//! stray byte can neither freeze the show nor swallow the next key.

use std::io;
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

/// How long a lone `ESC` (or an unfinished escape sequence) is kept before it
/// is treated as a key of its own.
const PARTIAL_TIMEOUT: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Esc,
    Enter,
    Tab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    /// A key we do not model (function keys, mouse reports, ...).
    Other,
    /// The terminal went away.
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyPress {
    pub const fn plain(key: Key) -> Self {
        Self {
            key,
            ctrl: false,
            alt: false,
        }
    }
}

pub struct Input {
    fd: RawFd,
    owns_fd: bool,
    buf: Vec<u8>,
    partial_since: Option<Instant>,
}

impl Input {
    /// Open the terminal for reading. Uses stdin when it is a tty and falls
    /// back to `/dev/tty` when input is redirected.
    pub fn new() -> io::Result<Self> {
        let stdin_is_tty = unsafe { libc::isatty(0) } == 1;
        let (fd, owns_fd) = if stdin_is_tty {
            (0, false)
        } else {
            let fd = unsafe { libc::open(c"/dev/tty".as_ptr(), libc::O_RDONLY) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            (fd, true)
        };
        Ok(Self {
            fd,
            owns_fd,
            buf: Vec::with_capacity(64),
            partial_since: None,
        })
    }

    /// Read whatever input is available and decode the complete key presses.
    /// This never blocks.
    pub fn poll(&mut self) -> Vec<KeyPress> {
        let mut pfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pfd, 1, 0) };
        if ready > 0 {
            if pfd.revents & libc::POLLIN != 0 {
                let mut chunk = [0u8; 512];
                let n = unsafe { libc::read(self.fd, chunk.as_mut_ptr().cast(), chunk.len()) };
                if n > 0 {
                    self.buf.extend_from_slice(&chunk[..n as usize]);
                } else if n == 0 {
                    return vec![KeyPress::plain(Key::Eof)];
                }
            } else if pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return vec![KeyPress::plain(Key::Eof)];
            }
        }
        self.parse()
    }

    fn parse(&mut self) -> Vec<KeyPress> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.buf.len() {
            let b = self.buf[i];

            if b == 0x1b {
                let Some(&next) = self.buf.get(i + 1) else {
                    // A lone ESC at the end: give the rest of the sequence a
                    // moment to arrive, then report the ESC key itself.
                    let since = *self.partial_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= PARTIAL_TIMEOUT {
                        out.push(KeyPress::plain(Key::Esc));
                        self.partial_since = None;
                        i += 1;
                    }
                    break;
                };

                if next == b'[' || next == b'O' {
                    let finish = self.buf[i + 2..]
                        .iter()
                        .position(|c| (0x40..=0x7e).contains(c))
                        .map(|p| i + 2 + p);
                    match finish {
                        Some(j) => {
                            let key = match self.buf[j] {
                                b'A' => Key::Up,
                                b'B' => Key::Down,
                                b'C' => Key::Right,
                                b'D' => Key::Left,
                                _ => Key::Other,
                            };
                            out.push(KeyPress::plain(key));
                            self.partial_since = None;
                            i = j + 1;
                        }
                        None => {
                            // Incomplete. Wait, but never forever: if the rest
                            // never arrives, drop it so the next key works.
                            let since = *self.partial_since.get_or_insert_with(Instant::now);
                            if since.elapsed() >= PARTIAL_TIMEOUT {
                                out.push(KeyPress::plain(Key::Other));
                                self.partial_since = None;
                                i = self.buf.len();
                            }
                            break;
                        }
                    }
                } else {
                    // Alt+key.
                    let mut press = byte_key(next);
                    press.alt = true;
                    out.push(press);
                    self.partial_since = None;
                    i += 2;
                }
                continue;
            }

            match b {
                // Multi-byte UTF-8: report one event and skip the rest.
                0x80..=0xbf => {
                    i += 1;
                    continue;
                }
                0xc0..=0xf7 => {
                    let extra = match b {
                        0xc0..=0xdf => 1,
                        0xe0..=0xef => 2,
                        _ => 3,
                    };
                    out.push(KeyPress::plain(Key::Other));
                    i = (i + 1 + extra).min(self.buf.len());
                    continue;
                }
                _ => {
                    out.push(byte_key(b));
                    i += 1;
                }
            }
        }

        self.buf.drain(..i);
        out
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        if self.owns_fd {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

fn byte_key(b: u8) -> KeyPress {
    match b {
        0x0d | 0x0a => KeyPress::plain(Key::Enter),
        0x09 => KeyPress::plain(Key::Tab),
        0x7f | 0x08 => KeyPress::plain(Key::Backspace),
        0x01..=0x1a => KeyPress {
            key: Key::Char((b'a' + b - 1) as char),
            ctrl: true,
            alt: false,
        },
        0x20..=0x7e => KeyPress::plain(Key::Char(b as char)),
        _ => KeyPress::plain(Key::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_with(bytes: &[u8]) -> Input {
        Input {
            fd: -1,
            owns_fd: false,
            buf: bytes.to_vec(),
            partial_since: None,
        }
    }

    #[test]
    fn decodes_plain_and_control_keys() {
        let mut input = input_with(b"q\x03\x0d\x7f");
        let keys = input.parse();
        assert_eq!(keys[0], KeyPress::plain(Key::Char('q')));
        assert_eq!(
            keys[1],
            KeyPress {
                key: Key::Char('c'),
                ctrl: true,
                alt: false
            }
        );
        assert_eq!(keys[2], KeyPress::plain(Key::Enter));
        assert_eq!(keys[3], KeyPress::plain(Key::Backspace));
    }

    #[test]
    fn decodes_arrows() {
        let mut input = input_with(b"\x1b[A\x1bOB\x1b[1;5C");
        let keys = input.parse();
        assert_eq!(keys[0], KeyPress::plain(Key::Up));
        assert_eq!(keys[1], KeyPress::plain(Key::Down));
        // Modifier parameters are ignored; the final byte still says Right.
        assert_eq!(keys[2], KeyPress::plain(Key::Right));
    }

    #[test]
    fn keeps_a_partial_sequence_buffered() {
        // ESC [ 1 ; arrives without a final byte: nothing is consumed yet.
        let mut input = input_with(b"\x1b[1;");
        let keys = input.parse();
        assert!(keys.is_empty());
        assert_eq!(input.buf, b"\x1b[1;");
    }

    #[test]
    fn a_partial_sequence_is_dropped_after_the_timeout() {
        let mut input = input_with(b"\x1b[1;");
        input.partial_since = Some(Instant::now() - Duration::from_millis(100));
        let keys = input.parse();
        assert_eq!(keys, vec![KeyPress::plain(Key::Other)]);
        assert!(input.buf.is_empty(), "garbage must not linger");
    }

    #[test]
    fn a_lone_escape_becomes_esc_after_a_moment() {
        let mut input = input_with(b"\x1b");
        assert!(input.parse().is_empty());
        input.partial_since = Some(Instant::now() - Duration::from_millis(100));
        assert_eq!(input.parse(), vec![KeyPress::plain(Key::Esc)]);
    }

    #[test]
    fn a_stray_partial_does_not_swallow_the_next_key() {
        // This is the ssh freeze: ESC [ arrives, then 'q' on its own later.
        let mut input = input_with(b"\x1b[");
        assert!(input.parse().is_empty());
        input.partial_since = Some(Instant::now() - Duration::from_millis(100));
        assert_eq!(input.parse(), vec![KeyPress::plain(Key::Other)]);
        input.buf.extend_from_slice(b"q");
        assert_eq!(input.parse(), vec![KeyPress::plain(Key::Char('q'))]);
    }

    #[test]
    fn decodes_utf8_as_one_event() {
        let mut input = input_with("é".as_bytes());
        let keys = input.parse();
        assert_eq!(keys, vec![KeyPress::plain(Key::Other)]);
        assert!(input.buf.is_empty());
    }
}
