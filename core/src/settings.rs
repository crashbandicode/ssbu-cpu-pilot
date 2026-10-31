//! The three user settings plus the CPU level, shared lock-free between the
//! menu callback thread and the game thread, and their INI form.
use core::sync::atomic::{AtomicU32, Ordering};

pub const LEVEL_MIN: u8 = 1;
pub const LEVEL_MAX: u8 = 9;
pub const DEFAULT_LEVEL: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// P1 is driven by the CPU.
    pub pilot: bool,
    /// CPU level 1..=9 (clamped).
    pub level: u8,
    pub auto_rematch: bool,
    pub auto_save_replay: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pilot: false,
            level: DEFAULT_LEVEL,
            auto_rematch: false,
            auto_save_replay: false,
        }
    }
}

pub const fn clamp_level(level: u8) -> u8 {
    if level < LEVEL_MIN {
        LEVEL_MIN
    } else if level > LEVEL_MAX {
        LEVEL_MAX
    } else {
        level
    }
}

impl Settings {
    pub const fn pack(self) -> u32 {
        (self.pilot as u32)
            | ((self.auto_rematch as u32) << 1)
            | ((self.auto_save_replay as u32) << 2)
            | ((clamp_level(self.level) as u32) << 8)
    }

    pub const fn unpack(bits: u32) -> Self {
        Self {
            pilot: bits & 1 != 0,
            auto_rematch: bits & 2 != 0,
            auto_save_replay: bits & 4 != 0,
            level: clamp_level(((bits >> 8) & 0xff) as u8),
        }
    }
}

/// Lock-free holder. Writers use `update`, which retries on contention.
pub struct SharedSettings(AtomicU32);

impl SharedSettings {
    pub const fn new(value: Settings) -> Self {
        Self(AtomicU32::new(value.pack()))
    }

    pub fn load(&self) -> Settings {
        Settings::unpack(self.0.load(Ordering::Acquire))
    }

    pub fn store(&self, value: Settings) {
        self.0.store(value.pack(), Ordering::Release);
    }

    /// Applies `f` atomically and returns the new value.
    pub fn update(&self, f: impl Fn(Settings) -> Settings) -> Settings {
        let mut current = self.0.load(Ordering::Acquire);
        loop {
            let next = f(Settings::unpack(current)).pack();
            match self
                .0
                .compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Settings::unpack(next),
                Err(actual) => current = actual,
            }
        }
    }
}

/// INI keys. They are also the Deluxe option ids, so the host's one-key
/// rewrite and this parser agree.
pub const KEY_PILOT: &str = "p1";
pub const KEY_LEVEL: &str = "level";
pub const KEY_REMATCH: &str = "auto_rematch";
pub const KEY_SAVE: &str = "auto_save_replay";

fn flag(value: &str) -> Option<bool> {
    let value = value.trim();
    [("on", true), ("off", false), ("cpu", true), ("player", false), ("1", true), ("0", false)]
        .iter()
        .find(|(name, _)| value.eq_ignore_ascii_case(name))
        .map(|&(_, on)| on)
}

fn level(value: &str) -> Option<u8> {
    let value = value.trim();
    let digits = value
        .strip_prefix("Lv")
        .or_else(|| value.strip_prefix("lv"))
        .unwrap_or(value);
    let n: u8 = digits.trim().parse().ok()?;
    (LEVEL_MIN..=LEVEL_MAX).contains(&n).then_some(n)
}

/// Parses `key=value` lines. Unknown keys, comments and bad values are
/// ignored; each missing or bad key keeps its default.
pub fn parse_ini(text: &str) -> Settings {
    let mut out = Settings::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with(['#', ';', '[']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            KEY_PILOT => out.pilot = flag(value).unwrap_or(out.pilot),
            KEY_LEVEL => out.level = level(value).unwrap_or(out.level),
            KEY_REMATCH => out.auto_rematch = flag(value).unwrap_or(out.auto_rematch),
            KEY_SAVE => out.auto_save_replay = flag(value).unwrap_or(out.auto_save_replay),
            _ => {}
        }
    }
    out
}

/// Writes the INI form into `buf` (no heap) and returns the length used, or
/// `None` if `buf` is too small. 96 bytes always suffice.
pub fn write_ini(value: Settings, buf: &mut [u8]) -> Option<usize> {
    let on = |b: bool| if b { "on" } else { "off" };
    let level = [b'0' + clamp_level(value.level)];
    let parts: [&[u8]; 12] = [
        KEY_PILOT.as_bytes(),
        b"=",
        if value.pilot { b"cpu\n" } else { b"player\n" },
        KEY_LEVEL.as_bytes(),
        b"=",
        &level,
        b"\nauto_rematch=",
        on(value.auto_rematch).as_bytes(),
        b"\n",
        KEY_SAVE.as_bytes(),
        b"=",
        on(value.auto_save_replay).as_bytes(),
    ];
    let mut at = 0;
    for part in parts.iter().chain([&b"\n"[..]].iter()) {
        let end = at + part.len();
        buf.get_mut(at..end)?.copy_from_slice(part);
        at = end;
    }
    Some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_round_trips_every_value() {
        for bits in 0..8u32 {
            for level in LEVEL_MIN..=LEVEL_MAX {
                let s = Settings {
                    pilot: bits & 1 != 0,
                    auto_rematch: bits & 2 != 0,
                    auto_save_replay: bits & 4 != 0,
                    level,
                };
                assert_eq!(Settings::unpack(s.pack()), s);
            }
        }
    }

    #[test]
    fn unpack_clamps_level() {
        assert_eq!(Settings::unpack(0).level, LEVEL_MIN);
        assert_eq!(Settings::unpack(200 << 8).level, LEVEL_MAX);
    }

    #[test]
    fn update_is_atomic_read_modify_write() {
        let shared = SharedSettings::new(Settings::default());
        let after = shared.update(|s| Settings { pilot: true, ..s });
        assert!(after.pilot && shared.load().pilot);
        assert_eq!(shared.load().level, DEFAULT_LEVEL);
    }

    #[test]
    fn ini_round_trips() {
        let s = Settings {
            pilot: true,
            level: 7,
            auto_rematch: true,
            auto_save_replay: false,
        };
        let mut buf = [0u8; 96];
        let n = write_ini(s, &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert_eq!(
            text,
            "p1=cpu\nlevel=7\nauto_rematch=on\nauto_save_replay=off\n"
        );
        assert_eq!(parse_ini(text), s);
    }

    #[test]
    fn ini_accepts_labels_indices_and_ignores_junk() {
        let s = parse_ini("# c\n[x]\np1 = CPU\nlevel=Lv9\nauto_rematch=1\nauto_save_replay=maybe\nzzz=1\nnoequals\n");
        assert_eq!(
            s,
            Settings {
                pilot: true,
                level: 9,
                auto_rematch: true,
                auto_save_replay: false,
            }
        );
        assert_eq!(parse_ini("level=0\nlevel=10"), Settings::default());
    }

    #[test]
    fn write_ini_refuses_small_buffer() {
        assert_eq!(write_ini(Settings::default(), &mut [0u8; 10]), None);
    }
}
