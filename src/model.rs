//! Pure, host-tested plugin facts: the supported game version, the plugin's
//! identity and data paths, the 13.0.5 site table with each site's 64 bytes,
//! and a fixed-buffer text line.
use crate::chain;

pub const SUPPORTED_VERSION: &str = "13.0.5";
pub const PLUGIN: &str = "ssbu-cpu-pilot";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DATA_DIR: &str = "sd:/ultimate/ssbu-cpu-pilot";
pub const CONFIG_INI: &str = "sd:/ultimate/ssbu-cpu-pilot/config.ini";

/// True only for the exact supported display version.
pub fn version_supported(display: &str) -> bool {
    display.trim_end_matches('\0').trim() == SUPPORTED_VERSION
}

/// Raw controller button bits (`Controller::current_buttons`, `+0x8`).
pub mod buttons {
    pub const X: u32 = 1 << 4;
    pub const L: u32 = 1 << 8;
    pub const R: u32 = 1 << 9;
    pub const COMBO: u32 = X | L | R;

    pub const fn combo_held(raw: u32) -> bool {
        raw & COMBO == COMBO
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// This plugin puts an inline hook here.
    Hooked,
    /// This plugin calls the function; other plugins' hooks inside it are fine.
    Called,
}

pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub kind: Kind,
    /// The 13.0.5 bytes at `offset` (`tools/site_bytes.py`).
    pub guard: [u8; 64],
}

impl Site {
    pub const fn stock(&self) -> [u32; chain::WORDS] {
        chain::words(&self.guard)
    }

    /// Whether a called function can be used as it is now.
    pub fn usable(&self, words: &[u32; chain::WORDS], address: u64) -> bool {
        chain::usable(&self.stock(), words, address, 0)
    }
}

/// Final input mapping `0x1750dd0`, after the player-index check
/// (`sxtw x23, w1`): reached only when the index is not -1, so it marks a
/// call whose `x19` (`out`) and `x23` (index) the exit can trust.
pub const FIM_VALID: Site = Site {
    name: "fim_valid",
    offset: 0x1750df0,
    kind: Kind::Hooked,
    guard: [
        0x37, 0x7c, 0x40, 0x93, 0xff, 0x22, 0x00, 0xf1, 0x62, 0x9f, 0x00, 0x54, 0xf3, 0x03, 0x02, 0xaa,
        0x5f, 0x00, 0x00, 0xf9, 0x23, 0x77, 0x00, 0xb4, 0x74, 0x08, 0x40, 0xf9, 0x82, 0x9e, 0x40, 0xb9,
        0x48, 0x04, 0x00, 0x51, 0xf5, 0x03, 0x00, 0xaa, 0x1f, 0x15, 0x00, 0x71, 0x28, 0x04, 0x00, 0x54,
        0x69, 0x06, 0x80, 0x52, 0x28, 0x25, 0xc8, 0x1a, 0x68, 0x06, 0x00, 0x37, 0x48, 0x0c, 0x00, 0x51,
    ],
};

/// Final input mapping's single exit (`ldp x29, x30, [sp, #0x70]`): `*out`
/// is complete, `x19` = out, `x23` = player index, `x20` = the controller.
pub const FIM_EXIT: Site = Site {
    name: "fim_exit",
    offset: 0x1751ce8,
    kind: Kind::Hooked,
    guard: [
        0xfd, 0x7b, 0x47, 0xa9, 0xf7, 0x23, 0x40, 0xf9, 0xf4, 0x4f, 0x46, 0xa9, 0xf6, 0x57, 0x45, 0xa9,
        0xff, 0x03, 0x02, 0x91, 0xc0, 0x03, 0x5f, 0xd6, 0xe9, 0x03, 0x00, 0x32, 0x29, 0x21, 0xc8, 0x1a,
        0x1f, 0x09, 0x00, 0x71, 0xc0, 0x1d, 0x00, 0x54, 0x1f, 0x0d, 0x00, 0x71, 0x21, 0x1e, 0x00, 0x54,
        0xf6, 0x03, 0x1f, 0x2a, 0xe8, 0x03, 0x00, 0x32, 0x9f, 0x01, 0x0e, 0x6a, 0x21, 0x97, 0xff, 0x54,
    ],
};

/// The AI's per-frame think (the Training Modpack hooks its entry).
pub const AI_THINK: Site = Site {
    name: "ai_think",
    offset: 0x2da180,
    kind: Kind::Called,
    guard: [
        0xff, 0x03, 0x06, 0xd1, 0xee, 0x73, 0x00, 0xfd, 0xed, 0x33, 0x0f, 0x6d, 0xeb, 0x2b, 0x10, 0x6d,
        0xe9, 0x23, 0x11, 0x6d, 0xfc, 0x6f, 0x12, 0xa9, 0xfa, 0x67, 0x13, 0xa9, 0xf8, 0x5f, 0x14, 0xa9,
        0xf6, 0x57, 0x15, 0xa9, 0xf4, 0x4f, 0x16, 0xa9, 0xfd, 0x7b, 0x17, 0xa9, 0xfd, 0xc3, 0x05, 0x91,
        0x68, 0x7e, 0x02, 0xd0, 0x08, 0x41, 0x45, 0xf9, 0x00, 0x01, 0xc0, 0x3d, 0x00, 0x0c, 0x83, 0x3d,
    ],
};

/// Sets a controller's AI rank.
pub const AI_SET_RANK: Site = Site {
    name: "ai_set_rank",
    offset: 0x6b8400,
    kind: Kind::Called,
    guard: [
        0xf5, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91,
        0xe8, 0x5f, 0x02, 0xd0, 0x08, 0xed, 0x47, 0xf9, 0xf3, 0x03, 0x01, 0x2a, 0xf4, 0x03, 0x00, 0xaa,
        0xa8, 0x03, 0x00, 0xb4, 0x95, 0x2e, 0x40, 0xf9, 0xa8, 0x0e, 0x4c, 0xb9, 0x1f, 0x01, 0x13, 0x6b,
        0x20, 0x03, 0x00, 0x54, 0x60, 0x02, 0x22, 0x1e, 0x01, 0x90, 0x36, 0x1e, 0x01, 0x28, 0x21, 0x1e,
    ],
};

/// Re-initializes an AI's scripts.
pub const AI_RESET: Site = Site {
    name: "ai_reset",
    offset: 0x2d8d40,
    kind: Kind::Called,
    guard: [
        0xf7, 0x0f, 0x1c, 0xf8, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b, 0x03, 0xa9,
        0xfd, 0xc3, 0x00, 0x91, 0xf4, 0x03, 0x01, 0x2a, 0x02, 0x00, 0x80, 0x12, 0xe1, 0x03, 0x1f, 0x2a,
        0xe3, 0x03, 0x1f, 0x2a, 0xf3, 0x03, 0x00, 0xaa, 0x62, 0x00, 0x00, 0x94, 0x68, 0x62, 0x41, 0xb9,
        0x69, 0x12, 0x32, 0x91, 0x7f, 0x66, 0x0c, 0xb9, 0x7f, 0xfa, 0x18, 0x79, 0x7f, 0xda, 0x18, 0x79,
    ],
};

pub const SITES: [&Site; 5] = [&FIM_VALID, &FIM_EXIT, &AI_THINK, &AI_SET_RANK, &AI_RESET];

/// A text line built in a fixed buffer; anything past `N` bytes is dropped.
pub struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    pub const fn new() -> Self {
        Self { buf: [0; N], len: 0 }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The text plus a NUL, for `nn::fs` paths; `None` when there is no room.
    pub fn with_nul(&mut self) -> Option<&[u8]> {
        if self.len >= N {
            return None;
        }
        self.buf[self.len] = 0;
        Some(&self.buf[..=self.len])
    }
}

impl<const N: usize> Default for Line<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> core::fmt::Write for Line<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write as _;

    #[test]
    fn version() {
        assert!(version_supported("13.0.5"));
        assert!(version_supported("13.0.5\0\0"));
        assert!(!version_supported("13.0.4"));
        assert!(!version_supported("13.0.50"));
        assert!(!version_supported(""));
    }

    #[test]
    fn combo_needs_all_three() {
        assert!(buttons::combo_held(buttons::COMBO | 1));
        assert!(!buttons::combo_held(buttons::L | buttons::R));
        assert!(!buttons::combo_held(buttons::X | buttons::L));
    }

    #[test]
    fn hooked_sites_hold_the_instructions_the_hooks_rely_on() {
        // sxtw x23, w1 ; cmp x23, #8
        assert_eq!(FIM_VALID.stock()[..2], [0x9340_7c37, 0xf100_22ff]);
        // ldp x29, x30, [sp, #0x70] ... add sp, sp, #0x80 ; ret
        assert_eq!(FIM_EXIT.stock()[0], 0xa947_7bfd);
        assert_eq!(FIM_EXIT.stock()[4..6], [0x9102_03ff, 0xd65f_03c0]);
    }

    #[test]
    fn called_guards_agree_with_the_forks_body_checks() {
        // The Training Modpack fork checks 8 body bytes at +0x20 of each.
        assert_eq!(AI_THINK.guard[0x20..0x28], [0xf6, 0x57, 0x15, 0xa9, 0xf4, 0x4f, 0x16, 0xa9]);
        assert_eq!(AI_SET_RANK.guard[0x20..0x28], [0xa8, 0x03, 0x00, 0xb4, 0x95, 0x2e, 0x40, 0xf9]);
    }

    #[test]
    fn stock_sites_are_usable_and_distinct() {
        for site in SITES {
            if site.kind == Kind::Called {
                assert!(site.usable(&site.stock(), 0x1000_0000 + site.offset as u64), "{}", site.name);
            }
        }
        let mut offsets: Vec<usize> = SITES.iter().map(|s| s.offset).collect();
        offsets.sort_unstable();
        offsets.dedup();
        assert_eq!(offsets.len(), SITES.len());
    }

    #[test]
    fn line_truncates_and_terminates() {
        let mut line = Line::<8>::new();
        let _ = write!(line, "abc{}", 12345);
        assert_eq!(line.as_bytes(), b"abc12345");
        assert_eq!(line.with_nul(), None);
        line.clear();
        let _ = write!(line, "ok");
        assert_eq!(line.with_nul(), Some(&b"ok\0"[..]));
    }
}
