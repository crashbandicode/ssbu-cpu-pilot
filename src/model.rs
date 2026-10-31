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
    /// Read-only instruction/data-layout dependency; never patched or called.
    Relied,
}

pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub kind: Kind,
    /// Only the HUD mark uses this site: a mismatch turns the mark off
    /// (`hud=off`), never the pilot.
    pub hud: bool,
    /// The 13.0.5 bytes at `offset` (`tools/site_bytes.py`).
    pub guard: [u8; 64],
}

impl Site {
    pub const fn stock(&self) -> [u32; chain::WORDS] {
        chain::words(&self.guard)
    }

    /// Whether a called function can be used as it is now.
    pub fn usable(&self, words: &[u32; chain::WORDS], address: u64) -> bool {
        let kept = if self.kind == Kind::Relied { chain::WORDS } else { 0 };
        chain::usable(&self.stock(), words, address, kept)
    }
}

/// Final input mapping `0x1750dd0`, after the player-index check
/// (`sxtw x23, w1`): reached only when the index is not -1, so it marks a
/// call whose `x19` (`out`) and `x23` (index) the exit can trust.
pub const FIM_VALID: Site = Site {
    name: "fim_valid",
    offset: 0x1750df0,
    kind: Kind::Hooked,
    hud: false,
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
    hud: false,
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
    hud: false,
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
    hud: false,
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
    hud: false,
    guard: [
        0xf7, 0x0f, 0x1c, 0xf8, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b, 0x03, 0xa9,
        0xfd, 0xc3, 0x00, 0x91, 0xf4, 0x03, 0x01, 0x2a, 0x02, 0x00, 0x80, 0x12, 0xe1, 0x03, 0x1f, 0x2a,
        0xe3, 0x03, 0x1f, 0x2a, 0xf3, 0x03, 0x00, 0xaa, 0x62, 0x00, 0x00, 0x94, 0x68, 0x62, 0x41, 0xb9,
        0x69, 0x12, 0x32, 0x91, 0x7f, 0x66, 0x0c, 0xb9, 0x7f, 0xfa, 0x18, 0x79, 0x7f, 0xda, 0x18, 0x79,
    ],
};

/// The UI manager's per-frame pass (`0x375e260`) over its holders, on the
/// UI job's thread: just before the holder's animate/calculate set-up, with
/// `x23` = the holder (`+0x458` its `nn::ui2d::Layout*`). The five words
/// `0x375ef8c..0x375efa0` are plain instructions, and a hook there ends before
/// the Training Modpack's pinned anchor at `0x375efa8` (its five-word pin and
/// the locator needle after it), so the two plugins' checks do not meet.
pub const HUD_PASS: Site = Site {
    name: "hud_pass",
    offset: 0x375ef8c,
    kind: Kind::Hooked,
    hud: true,
    guard: [
        0x08, 0x5d, 0x16, 0x9b, 0xf3, 0x42, 0x01, 0x91, 0xe0, 0x03, 0x13, 0xaa, 0x08, 0x41, 0x0b, 0x91,
        0xe1, 0x03, 0x1f, 0x2a, 0xe8, 0xa2, 0x11, 0xa9, 0x43, 0x97, 0x23, 0x97, 0xe0, 0x2e, 0x42, 0xf9,
        0x08, 0x00, 0x40, 0xf9, 0x08, 0x4d, 0x40, 0xf9, 0xe1, 0x03, 0x13, 0xaa, 0xe2, 0x03, 0x1f, 0x2a,
        0x00, 0x01, 0x3f, 0xd6, 0xe0, 0x03, 0x13, 0xaa, 0x4f, 0x97, 0x23, 0x97, 0xe0, 0x03, 0x13, 0xaa,
    ],
};

/// `nn::ui2d::Pane::FindPaneByName(pane, name)` over the pane's subtree
/// (a Parts pane's layout included); no allocation, unlike
/// `Layout::GetPaneByName` (`0x3776910`). The Training Modpack and
/// smash-gamepad-overlay call it.
pub const FIND_PANE: Site = Site {
    name: "find_pane",
    offset: 0x59970,
    kind: Kind::Called,
    hud: true,
    guard: [
        0xf5, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91,
        0xf3, 0x03, 0x01, 0xaa, 0xe8, 0x03, 0x1f, 0xaa, 0x09, 0x00, 0x08, 0x8b, 0x29, 0xc1, 0x42, 0x39,
        0x6a, 0x6a, 0x68, 0x38, 0x3f, 0x01, 0x0a, 0x6b, 0xc1, 0x00, 0x00, 0x54, 0x69, 0x02, 0x00, 0x34,
        0x08, 0x05, 0x00, 0x91, 0x1f, 0x61, 0x00, 0xf1, 0x03, 0xff, 0xff, 0x54, 0x0f, 0x00, 0x00, 0x14,
    ],
};

/// Pane text setter `(pane, text, units, flags)` for a UTF-16 text; every
/// pane text passes it. Called, never hooked: it grows the pane's buffer to
/// fit before writing (as gsp-visualizer calls it).
pub const SET_TEXT: Site = Site {
    name: "set_text",
    offset: 0x37a2270,
    kind: Kind::Called,
    hud: true,
    guard: [
        0xfc, 0x57, 0xbd, 0xa9, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91,
        0xff, 0x0b, 0x40, 0xd1, 0xff, 0x43, 0x00, 0xd1, 0xf4, 0x03, 0x01, 0xaa, 0xf3, 0x03, 0x00, 0xaa,
        0xbf, 0x83, 0x1d, 0xf8, 0xe3, 0x0b, 0x00, 0x36, 0xbf, 0x83, 0x1d, 0xf8, 0x74, 0x0f, 0x00, 0xb4,
        0x8a, 0x02, 0x40, 0x79, 0x2a, 0x0f, 0x00, 0x34, 0xe8, 0x03, 0x1f, 0x2a, 0xe9, 0x07, 0x00, 0x32,
    ],
};


// ---------------------------------------------------------------------------
// The HUD mark: P1's tag text reads "CPU Lv<n>" while the CPU drives P1.

/// The match HUD's layout (`Layout::layout_name`).
pub const HUD_LAYOUT: &[u8] = b"info_melee";
/// The Parts panes of each player slot 0..=3 in the HUD layout: the
/// four-player and the compact variant (`p<n>` uses
/// `info_melee_lct_player_00`, `p<n>_2` uses `_01`), each holding
/// `RootPane/set_txt_00` (the game writes both).
const HUD_PARTS: [[&[u8]; 2]; 4] = [
    [b"p1\0", b"p1_2\0"],
    [b"p2\0", b"p2_2\0"],
    [b"p3\0", b"p3_2\0"],
    [b"p4\0", b"p4_2\0"],
];

/// The Parts panes holding the tag of the piloted player's slot (online the
/// local player may be slot 1); `None` beyond four players.
pub fn hud_parts(slot: u8) -> Option<[&'static [u8]; 2]> {
    HUD_PARTS.get(usize::from(slot)).copied()
}

/// The HUD's wanted state in one byte: 0 = the game's text, else the level
/// (1..=9) and the slot (0..=3) whose tag shows it.
pub const fn hud_wanted(level: u8, slot: u8) -> u8 {
    if level == 0 || slot > 3 {
        0
    } else {
        (level & 0x0f) | (slot << 4)
    }
}

/// `(level, slot)` of a `hud_wanted` byte.
pub const fn hud_split(wanted: u8) -> (u8, u8) {
    (wanted & 0x0f, wanted >> 4)
}
/// The tag TextBox inside a player's Parts layout.
pub const HUD_TAG_PANE: &[u8] = b"set_txt_00\0";

/// Object fields read from the game's memory (13.0.5).
pub mod ui2d {
    /// UI manager holder -> `nn::ui2d::Layout*` (the Training Modpack's
    /// registry reads the same field, `ui_anchor::HOLDER_LAYOUT`).
    pub const HOLDER_LAYOUT: usize = 0x458;
    /// `Layout::root_pane` (nnsdk-rs `Layout`).
    pub const LAYOUT_ROOT: usize = 0x18;
    /// `Layout::layout_name`, a C string.
    pub const LAYOUT_NAME: usize = 0x30;
    /// `TextBox::text_buf`, the UTF-16 string (gsp-visualizer reads it).
    pub const TEXT_BUF: usize = 0xd8;
    /// `TextBox::text_buf_len`, the buffer's size in units.
    pub const TEXT_CAP: usize = 0x110;
    /// `TextBox::text_len`, the string's length in units.
    pub const TEXT_LEN: usize = 0x112;
    /// `TextBox::is_utf8`; the tag is read only as UTF-16.
    pub const TEXT_IS_UTF8: usize = 0x117;
}

/// Whether the NUL-terminated name read byte by byte through `byte_at` is
/// exactly `expected`; reads no further than the first difference or NUL.
pub fn name_is(mut byte_at: impl FnMut(usize) -> u8, expected: &[u8]) -> bool {
    expected.iter().enumerate().all(|(i, want)| byte_at(i) == *want) && byte_at(expected.len()) == 0
}

/// The longest tag text that is kept to be put back (UTF-16 units).
pub const TAG_CAP: usize = 64;
/// The label buffer's size (`ssbu_cpu_pilot_core::hud::label_utf16`).
pub const LABEL_CAP: usize = 16;

/// How many units of a TextBox's string to copy, from its fields, or `None`
/// when it is not a plain UTF-16 text this plugin can read back. Never more
/// than `TAG_CAP + 1`, which already tells `TagSlot::step` it is too long.
pub fn tag_text_units(is_utf8: u8, buf: usize, len: u16, cap: u16) -> Option<usize> {
    if is_utf8 != 0 || len > cap {
        return None;
    }
    if len == 0 {
        return Some(0);
    }
    (buf != 0 && buf.is_multiple_of(2)).then_some(usize::from(len).min(TAG_CAP + 1))
}

/// What to do with a tag pane's text.
#[derive(Debug, PartialEq, Eq)]
pub enum TagWrite<'a> {
    /// Leave the text as it is.
    Keep,
    /// Write the label that was passed in.
    Label,
    /// Write the game's own text back.
    Original(&'a [u16]),
}

/// The bookkeeping for one tag TextBox this plugin may have replaced: the
/// pane (its address, only compared, never read), the label on it and the
/// game's text before the first write.
pub struct TagSlot {
    pane: usize,
    shown: u8,
    original: [u16; TAG_CAP + 1],
    original_len: usize,
    label: [u16; LABEL_CAP],
    label_len: usize,
}

impl TagSlot {
    pub const fn new() -> Self {
        Self {
            pane: 0,
            shown: 0,
            original: [0; TAG_CAP + 1],
            original_len: 0,
            label: [0; LABEL_CAP],
            label_len: 0,
        }
    }

    /// The level of the label this slot put on its pane; 0 = none.
    pub fn shown(&self) -> u8 {
        self.shown
    }

    /// Drops the claim on the pane (it is gone, or not readable).
    pub fn forget(&mut self) {
        self.shown = 0;
        self.pane = 0;
    }

    fn ours(&self) -> &[u16] {
        &self.label[..self.label_len]
    }

    fn keep_label(&mut self, wanted: u8, label: &[u16]) {
        self.label[..label.len()].copy_from_slice(label);
        self.label_len = label.len();
        self.shown = wanted;
    }

    /// One look at the pane `pane` whose text now is `current`: `wanted` is
    /// the level to show (0 = the game's text) and `label` its text. The new
    /// state assumes the returned write is made.
    ///
    /// The game's text is kept before the first write and put back only while
    /// the pane still shows this plugin's label; if the game rewrote the text,
    /// or the pane is another object (a new match's HUD), it is left alone.
    pub fn step(&mut self, wanted: u8, pane: usize, current: &[u16], label: &[u16]) -> TagWrite<'_> {
        if wanted == 0 {
            let restore = self.shown != 0 && self.pane == pane && current == self.ours();
            self.forget();
            return if restore {
                TagWrite::Original(&self.original[..self.original_len])
            } else {
                TagWrite::Keep
            };
        }
        if label.is_empty() || label.len() >= LABEL_CAP {
            return TagWrite::Keep;
        }
        if self.shown != 0 && self.pane == pane && current == self.ours() {
            // Our label is still there: a new level keeps the saved original.
            if label == self.ours() {
                self.shown = wanted;
                return TagWrite::Keep;
            }
            self.keep_label(wanted, label);
            return TagWrite::Label;
        }
        // The first write on this pane, or the game has rewritten the text.
        if current.len() > TAG_CAP || current == label {
            self.forget();
            return TagWrite::Keep;
        }
        self.original[..current.len()].copy_from_slice(current);
        self.original[current.len()] = 0;
        self.original_len = current.len();
        self.pane = pane;
        self.keep_label(wanted, label);
        TagWrite::Label
    }
}

impl Default for TagSlot {
    fn default() -> Self {
        Self::new()
    }
}
/// UI builder single exit; x22 = records + 0x1f0.
pub const RESULTS_INPUT: Site = Site {
    name: "results_input",
    offset: 0x376309c,
    kind: Kind::Hooked,
    hud: false,
    guard: [
        0xfd, 0x7b, 0x47, 0xa9, 0xfb, 0x13, 0x40, 0xf9, 0xf4, 0x4f, 0x46, 0xa9, 0xf6, 0x57, 0x45, 0xa9,
        0xf8, 0x5f, 0x44, 0xa9, 0xfa, 0x67, 0x43, 0xa9, 0xff, 0x03, 0x02, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
        0x00, 0x00, 0x00, 0x00, 0xff, 0x43, 0x05, 0xd1, 0xfc, 0x7b, 0x00, 0xf9, 0xfa, 0x67, 0x10, 0xa9,
        0xf8, 0x5f, 0x11, 0xa9, 0xf6, 0x57, 0x12, 0xa9, 0xf4, 0x4f, 0x13, 0xa9, 0xfd, 0x7b, 0x14, 0xa9,
    ],
};

/// Cards update after prologue and helper; x19 = live cards.
pub const RESULTS_CARDS: Site = Site {
    name: "results_cards",
    offset: 0x1d59560,
    kind: Kind::Hooked,
    hud: false,
    guard: [
        0x68, 0xab, 0x01, 0x90, 0x15, 0x79, 0x40, 0xf9, 0x68, 0x16, 0x41, 0xb9, 0xf4, 0x03, 0x00, 0xaa,
        0x1f, 0x1d, 0x00, 0x71, 0xc8, 0x01, 0x00, 0x54, 0x09, 0x7d, 0x40, 0x93, 0xa9, 0x0a, 0x09, 0x8b,
        0x29, 0x99, 0x42, 0xb9, 0x3f, 0x05, 0x00, 0x31, 0x20, 0x01, 0x00, 0x54, 0x6a, 0x1a, 0x41, 0xb9,
        0x5f, 0x0d, 0x00, 0x71, 0xe9, 0x17, 0x9f, 0x1a, 0x5f, 0x05, 0x00, 0x71, 0xf7, 0x17, 0x9f, 0x1a,
    ],
};

/// Shared ordinary rematch parse; x20 = record, x19 = wrapper.
pub const RESULTS_REMATCH: Site = Site {
    name: "results_rematch",
    offset: 0x147fb64,
    kind: Kind::Hooked,
    hud: false,
    guard: [
        0x60, 0x86, 0x40, 0xa9, 0x12, 0xc8, 0x79, 0x94, 0xf3, 0x07, 0x40, 0xf9, 0xf3, 0xf6, 0xff, 0xb5,
        0xc2, 0xff, 0xff, 0x17, 0x68, 0x06, 0x40, 0xf9, 0x1f, 0x19, 0x01, 0xb9, 0xf3, 0x07, 0x40, 0xf9,
        0x53, 0xf6, 0xff, 0xb5, 0xbd, 0xff, 0xff, 0x17, 0xa0, 0xf4, 0x01, 0xb0, 0x00, 0x60, 0x13, 0x91,
        0xc7, 0x02, 0x95, 0x94, 0x60, 0xf2, 0xff, 0x34, 0x19, 0x91, 0x0b, 0x94, 0x20, 0x17, 0x00, 0x90,
    ],
};

/// Teardown deletes and nulls the results singleton at 0x52c35d0.
pub const RESULTS_LIFETIME: Site = Site {
    name: "results_lifetime",
    offset: 0x150ea38,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0xb3, 0xed, 0x01, 0xb0, 0x60, 0xea, 0x42, 0xf9, 0xa0, 0x00, 0x00, 0xb4, 0x08, 0x00, 0x40, 0xf9,
        0x08, 0x05, 0x40, 0xf9, 0x00, 0x01, 0x3f, 0xd6, 0x7f, 0xea, 0x02, 0xf9, 0xf3, 0x0b, 0x40, 0xf9,
        0xb3, 0x01, 0x00, 0xb4, 0x68, 0x22, 0x00, 0x91, 0x09, 0xfd, 0x5f, 0xc8, 0x2a, 0x05, 0x00, 0xd1,
        0x0a, 0xfd, 0x0b, 0xc8, 0xab, 0xff, 0xff, 0x35, 0xe9, 0x00, 0x00, 0xb5, 0x68, 0x02, 0x40, 0xf9,
    ],
};

/// Cards fighter index maps through [0x52c50f0]+0x298 to UI port.
pub const RESULTS_PORT: Site = Site {
    name: "results_port",
    offset: 0x1d59dc4,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x68, 0x6a, 0xb6, 0xb8, 0x1f, 0x1d, 0x00, 0x71, 0xe1, 0x03, 0x17, 0x2a, 0x68, 0x00, 0x00, 0x54,
        0xa8, 0x0a, 0x08, 0x8b, 0x01, 0x99, 0x42, 0xb9, 0xe0, 0x03, 0x13, 0xaa, 0xec, 0x05, 0x00, 0x94,
        0x9f, 0x1e, 0x00, 0xf1, 0x48, 0xfc, 0xff, 0x54, 0x94, 0x06, 0x00, 0x91, 0xd6, 0x02, 0x04, 0x91,
        0x80, 0xfe, 0x07, 0x36, 0xde, 0xff, 0xff, 0x17, 0x70, 0x1a, 0x41, 0xb9, 0x1f, 0x0e, 0x00, 0x71,
    ],
};

/// Save input consumes UI port stride 0x30, pressed bit 7 and modal gate.
pub const RESULTS_Y: Site = Site {
    name: "results_y",
    offset: 0x1d5b5a8,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x08, 0xdf, 0x01, 0x90, 0x08, 0xc5, 0x44, 0xf9, 0x08, 0x05, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xb9,
        0x1f, 0x05, 0x00, 0x71, 0x48, 0x01, 0x00, 0x54, 0x3f, 0x28, 0x00, 0x71, 0x82, 0x07, 0x00, 0x54,
        0x08, 0xdf, 0x01, 0x90, 0x08, 0xc1, 0x44, 0xf9, 0x08, 0x05, 0x40, 0xf9, 0xe9, 0x07, 0x1c, 0x32,
        0x28, 0x20, 0x29, 0x9b, 0x08, 0x11, 0x40, 0x39, 0xc8, 0x00, 0x38, 0x37, 0xe0, 0x03, 0x1f, 0x2a,
    ],
};

/// Save descriptor parse/open and status 1 publication.
pub const RESULTS_SAVE_OPEN: Site = Site {
    name: "results_save_open",
    offset: 0x1d5b650,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0xe1, 0x03, 0x14, 0xaa, 0xbf, 0xd6, 0x54, 0x94, 0xe0, 0x00, 0x00, 0x36, 0xa0, 0x0a, 0x40, 0xf9,
        0xe1, 0x03, 0x14, 0xaa, 0x43, 0x4a, 0x56, 0x94, 0xa0, 0x86, 0x40, 0xa9, 0x51, 0x59, 0x56, 0x94,
        0x03, 0x00, 0x00, 0x14, 0xa8, 0x06, 0x40, 0xf9, 0x1f, 0x19, 0x01, 0xb9, 0x48, 0xab, 0x01, 0x90,
        0x08, 0xe5, 0x42, 0xf9, 0x08, 0x05, 0x40, 0xf9, 0xe0, 0x03, 0x00, 0x32, 0x00, 0x11, 0x00, 0xb9,
    ],
};

/// Native button 2 causes UI status 2.
pub const RESULTS_SAVE_ACCEPT: Site = Site {
    name: "results_save_accept",
    offset: 0x1d5a590,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x88, 0x06, 0x40, 0xf9, 0x09, 0x19, 0x41, 0xb9, 0x68, 0xf6, 0x80, 0xb9, 0x3f, 0x09, 0x00, 0x71,
        0x21, 0x07, 0x00, 0x54, 0x1f, 0x4d, 0x00, 0x71, 0xc2, 0x0a, 0x00, 0x54, 0x68, 0x0a, 0x08, 0x8b,
        0xa9, 0x00, 0x80, 0x52, 0x09, 0xa9, 0x00, 0xb9, 0x48, 0xab, 0x01, 0xb0, 0x08, 0xe5, 0x42, 0xf9,
        0x08, 0x05, 0x40, 0xf9, 0xe9, 0x03, 0x1f, 0x32, 0x09, 0x11, 0x00, 0xb9, 0x54, 0xab, 0x01, 0xb0,
    ],
};

/// Save success record and status 3 publication.
pub const RESULTS_SAVE_SUCCESS: Site = Site {
    name: "results_save_success",
    offset: 0x1d5a840,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x94, 0xec, 0x8a, 0xd2, 0xb4, 0xdd, 0xa3, 0xf2, 0xd4, 0xe3, 0xdf, 0xf2, 0xf4, 0x1f, 0xeb, 0xf2,
        0xe1, 0x03, 0x14, 0xaa, 0x3f, 0xda, 0x54, 0x94, 0x20, 0x04, 0x00, 0x36, 0x60, 0x0a, 0x40, 0xf9,
        0xe1, 0x03, 0x14, 0xaa, 0xc3, 0x4d, 0x56, 0x94, 0x60, 0x86, 0x40, 0xa9, 0xd1, 0x5c, 0x56, 0x94,
        0xe8, 0x07, 0x00, 0x32, 0x21, 0x00, 0x00, 0x14, 0x68, 0xf6, 0x80, 0xb9, 0x1f, 0x4d, 0x00, 0x71,
    ],
};

/// Open copies input mode/default/cancel into popup; result reset.
pub const RESULTS_POPUP_FIELDS: Site = Site {
    name: "results_popup_fields",
    offset: 0x32f22c4,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x88, 0x72, 0x41, 0xb9, 0x68, 0x22, 0x01, 0xb9, 0x88, 0x62, 0x41, 0xb9, 0x68, 0x3e, 0x01, 0xb9,
        0x88, 0x6e, 0x41, 0xb9, 0x68, 0x1e, 0x01, 0xb9, 0x68, 0x1a, 0x01, 0xb9, 0x88, 0xd6, 0x45, 0x39,
        0x68, 0x12, 0x05, 0x39, 0x89, 0x02, 0x40, 0xb9, 0x29, 0x01, 0x1e, 0x32, 0x3f, 0x11, 0x00, 0x71,
        0xc1, 0x03, 0x00, 0x54, 0xe9, 0x03, 0x00, 0x32, 0x69, 0x26, 0x01, 0xb9, 0x89, 0x66, 0x41, 0xb9,
    ],
};

/// Selection setter writes the current button ID at +0x250.
pub const RESULTS_CURSOR: Site = Site {
    name: "results_cursor",
    offset: 0x376db18,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0x75, 0x52, 0x02, 0xb9, 0xc9, 0x07, 0x00, 0xb4, 0xe8, 0x03, 0x17, 0xaa, 0x2a, 0x21, 0x40, 0xb9,
        0x5f, 0x01, 0x15, 0x6b, 0xea, 0xa7, 0x9f, 0x1a, 0x08, 0xb1, 0x89, 0x9a, 0x29, 0x59, 0x6a, 0xf8,
        0x69, 0xff, 0xff, 0xb5, 0x1f, 0x01, 0x17, 0xeb, 0xa0, 0x06, 0x00, 0x54, 0x09, 0x21, 0x40, 0xb9,
        0x3f, 0x01, 0x15, 0x6b, 0x4c, 0x06, 0x00, 0x54, 0x15, 0x15, 0x40, 0xf9, 0x15, 0x06, 0x00, 0xb4,
    ],
};

/// Native decided/selection -> popup result, read only.
pub const RESULTS_POPUP_DECISION: Site = Site {
    name: "results_popup_decision",
    offset: 0x32f1630,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0xe8, 0x0a, 0x00, 0x34, 0x68, 0x8a, 0x40, 0xf9, 0x08, 0x41, 0x40, 0xf9, 0x09, 0x35, 0x45, 0x39,
        0x29, 0x0a, 0x00, 0x34, 0x08, 0x3d, 0x41, 0xb9, 0x09, 0x05, 0x00, 0x51, 0x3f, 0x0d, 0x00, 0x71,
        0x43, 0x16, 0x00, 0x54, 0x1f, 0x11, 0x00, 0x71, 0x61, 0x09, 0x00, 0x54, 0xe8, 0x01, 0x01, 0xd0,
        0x08, 0x69, 0x44, 0xf9, 0x00, 0x05, 0x40, 0xf9, 0x08, 0x24, 0x44, 0xa9, 0x29, 0x01, 0x08, 0xcb,
    ],
};

/// process_begin +0x60: x19 is the service owner, x29 is this native frame.
/// Beyond both Training Modpack's entry hook and its +0x30 body-byte check.
pub const AI_PROCESS_BEGIN: Site = Site {
    name: "ai_begin",
    offset: 0x30c770,
    kind: Kind::Hooked,
    hud: false,
    guard: [
        0xf4, 0x80, 0x02, 0xd0, 0x89, 0xee, 0x44, 0xf9, 0x29, 0x01, 0x40, 0xf9, 0x2a, 0x7d, 0x41, 0xb9,
        0x5f, 0x05, 0x00, 0x31, 0x00, 0x08, 0x00, 0x54, 0x28, 0x95, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9,
        0x20, 0xa1, 0x04, 0x91, 0x00, 0x01, 0x3f, 0xd6, 0x68, 0x02, 0x40, 0xf9, 0x09, 0x51, 0x41, 0xb9,
        0x1f, 0x00, 0x09, 0x6b, 0x00, 0x07, 0x00, 0x54, 0x00, 0x51, 0x01, 0xb9, 0x68, 0x02, 0x40, 0xf9,
    ],
};

/// All normal returns of process_begin reach this epilogue with x29 intact.
pub const AI_PROCESS_END: Site = Site {
    name: "ai_end",
    offset: 0x3119d0,
    kind: Kind::Hooked,
    hud: false,
    guard: [
        0xff, 0x83, 0x05, 0x91, 0xe9, 0x23, 0x43, 0x6d, 0xeb, 0x2b, 0x42, 0x6d, 0xed, 0x33, 0x41, 0x6d,
        0xfd, 0x7b, 0x49, 0xa9, 0xf4, 0x4f, 0x48, 0xa9, 0xf6, 0x57, 0x47, 0xa9, 0xf8, 0x5f, 0x46, 0xa9,
        0xfa, 0x67, 0x45, 0xa9, 0xfc, 0x6f, 0x44, 0xa9, 0xef, 0x3b, 0xca, 0x6c, 0xc0, 0x03, 0x5f, 0xd6,
        0x09, 0x01, 0x40, 0xf9, 0x01, 0xc0, 0x40, 0xbd, 0x03, 0x90, 0x22, 0x1e, 0x00, 0xe4, 0x00, 0x6f,
    ],
};

/// Native training predicate identifies the initialized and melee globals.
pub const MODE_GLOBALS: Site = Site {
    name: "mode_globals",
    offset: 0x1671650,
    kind: Kind::Relied,
    hud: false,
    guard: [
        0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x08, 0xe5, 0x01, 0xf0, 0x08, 0x61, 0x13, 0x91,
        0x08, 0xfd, 0xdf, 0x08, 0x48, 0x01, 0x00, 0x36, 0xa8, 0xe2, 0x01, 0x90, 0x08, 0xc1, 0x46, 0x39,
        0xe9, 0x03, 0x00, 0x32, 0x1f, 0x81, 0x00, 0x71, 0x2a, 0x01, 0x9f, 0x1a, 0x1f, 0x31, 0x00, 0x71,
        0x20, 0x01, 0x8a, 0x1a, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0xe5, 0x01, 0xf0,
    ],
};

/// The results code's name for the same mode predicate (initialized flag and
/// melee global, the fork's game_mode guard); one site, one guard.
pub const RESULTS_MODE: Site = MODE_GLOBALS;

pub const SITES: [&Site; 23] = [
    &FIM_VALID,
    &FIM_EXIT,
    &AI_THINK,
    &AI_SET_RANK,
    &AI_RESET,
    &HUD_PASS,
    &FIND_PANE,
    &SET_TEXT,
    &RESULTS_INPUT,
    &RESULTS_CARDS,
    &RESULTS_REMATCH,
    &RESULTS_LIFETIME,
    &RESULTS_PORT,
    &RESULTS_Y,
    &RESULTS_SAVE_OPEN,
    &RESULTS_SAVE_ACCEPT,
    &RESULTS_SAVE_SUCCESS,
    &RESULTS_POPUP_FIELDS,
    &RESULTS_CURSOR,
    &RESULTS_POPUP_DECISION,
    &AI_PROCESS_BEGIN,
    &AI_PROCESS_END,
    &MODE_GLOBALS,
];

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
    fn hud_follows_the_piloted_slot() {
        assert_eq!(hud_parts(0), Some([&b"p1\0"[..], &b"p1_2\0"[..]]));
        assert_eq!(hud_parts(1), Some([&b"p2\0"[..], &b"p2_2\0"[..]]));
        assert_eq!(hud_parts(4), None);
        assert_eq!(hud_wanted(0, 1), 0);
        assert_eq!(hud_wanted(3, 4), 0);
        for level in 1..=9 {
            for slot in 0..=3 {
                let w = hud_wanted(level, slot);
                assert_ne!(w, 0);
                assert_eq!(hud_split(w), (level, slot));
            }
        }
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
            if site.kind != Kind::Hooked {
                assert!(site.usable(&site.stock(), 0x1000_0000 + site.offset as u64), "{}", site.name);
            }
        }
        let mut offsets: Vec<usize> = SITES.iter().map(|s| s.offset).collect();
        offsets.sort_unstable();
        offsets.dedup();
        assert_eq!(offsets.len(), SITES.len());
    }

    #[test]
    fn hud_site_ends_before_the_modpacks_anchor() {
        // madd x8,x8,x22,x23 ; add x19,x23,#0x50 ; mov x0,x19 ; add x8,x8,#0x2d0 ; mov w1,wzr
        assert_eq!(
            HUD_PASS.stock()[..5],
            [0x9b16_5d08, 0x9101_42f3, 0xaa13_03e0, 0x910b_4108, 0x2a1f_03e1]
        );
        // No hook of this plugin reaches the Training Modpack's pinned word
        // `ldr x0, [x23, #0x458]` at 0x375efa8 (word 7 of the window).
        assert_eq!(HUD_PASS.stock()[7], 0xf942_2ee0);
        let end = HUD_PASS.offset + 4 * chain::hooked_words(HUD_PASS.offset as u64);
        assert!(end <= 0x375efa8, "{end:#x}");
        // And a pass hook by the Modpack inside the window is accepted.
        let mut words = HUD_PASS.stock();
        words[7] = 0x1400_0100; // b #0x400
        let at = 0x7100_0000 + HUD_PASS.offset as u64;
        assert_eq!(chain::entry(&HUD_PASS.stock(), &words, at), chain::Entry::Stock);
    }

    #[test]
    fn only_the_hud_sites_may_turn_the_mark_off() {
        let hud: Vec<&str> = SITES.iter().filter(|s| s.hud).map(|s| s.name).collect();
        assert_eq!(hud, ["hud_pass", "find_pane", "set_text"]);
        assert_eq!(HUD_PASS.kind, Kind::Hooked);
        assert_eq!(FIND_PANE.kind, Kind::Called);
        assert_eq!(SET_TEXT.kind, Kind::Called);
        // The called setter is the UTF-16 one gsp-visualizer calls; the
        // finder is a leaf that compares names at pane+0xb0 (24 bytes).
        assert_eq!(SET_TEXT.guard[..4], [0xfc, 0x57, 0xbd, 0xa9]);
        assert_eq!(FIND_PANE.guard[0x18..0x1c], [0x09, 0x00, 0x08, 0x8b]);
    }

    #[test]
    fn layout_names_are_compared_without_overreading() {
        let name = b"info_melee\0";
        let mut seen = 0;
        assert!(name_is(
            |i| {
                seen = seen.max(i + 1);
                name[i]
            },
            HUD_LAYOUT
        ));
        assert_eq!(seen, 11);
        // A shorter name stops at its NUL, a different one at the difference.
        let mut seen = 0;
        assert!(!name_is(
            |i| {
                seen = seen.max(i + 1);
                b"info\0"[i]
            },
            HUD_LAYOUT
        ));
        assert_eq!(seen, 5);
        let mut seen = 0;
        assert!(!name_is(
            |i| {
                seen = seen.max(i + 1);
                b"xnfo_melee\0"[i]
            },
            HUD_LAYOUT
        ));
        assert_eq!(seen, 1);
        // A longer name is not the layout either.
        assert!(!name_is(|i| b"info_melee_2\0"[i], HUD_LAYOUT));
    }

    #[test]
    fn tag_text_is_read_only_as_plain_utf16() {
        assert_eq!(tag_text_units(0, 0x1000, 2, 8), Some(2));
        assert_eq!(tag_text_units(0, 0, 0, 0), Some(0));
        assert_eq!(tag_text_units(1, 0x1000, 2, 8), None);
        assert_eq!(tag_text_units(0, 0x1000, 9, 8), None);
        assert_eq!(tag_text_units(0, 0, 2, 8), None);
        assert_eq!(tag_text_units(0, 0x1001, 2, 8), None);
        // Longer than the kept text: enough units to say so, no more.
        assert_eq!(tag_text_units(0, 0x1000, 500, 600), Some(TAG_CAP + 1));
    }

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn label(level: u8) -> Vec<u16> {
        let mut buf = [0u16; LABEL_CAP];
        let n = ssbu_cpu_pilot_core::hud::label_utf16(level, &mut buf);
        buf[..n].to_vec()
    }

    #[test]
    fn the_label_goes_on_and_the_original_comes_back() {
        let mut slot = TagSlot::new();
        let p1 = utf16("P1");
        assert_eq!(slot.step(3, 0x100, &p1, &label(3)), TagWrite::Label);
        assert_eq!(slot.shown(), 3);
        // Already there: no second write.
        assert_eq!(slot.step(3, 0x100, &label(3), &label(3)), TagWrite::Keep);
        // Back to the player: the game's text returns.
        assert_eq!(slot.step(0, 0x100, &label(3), &[]), TagWrite::Original(&p1[..]));
        assert_eq!(slot.shown(), 0);
        // Nothing left to put back.
        assert_eq!(slot.step(0, 0x100, &p1, &[]), TagWrite::Keep);
    }

    #[test]
    fn a_new_level_keeps_the_first_original() {
        let mut slot = TagSlot::new();
        let name = utf16("Mario");
        assert_eq!(slot.step(3, 7, &name, &label(3)), TagWrite::Label);
        assert_eq!(slot.step(5, 7, &label(3), &label(5)), TagWrite::Label);
        assert_eq!(slot.shown(), 5);
        assert_eq!(slot.step(0, 7, &label(5), &[]), TagWrite::Original(&name[..]));
    }

    #[test]
    fn an_empty_original_is_put_back_as_empty() {
        let mut slot = TagSlot::new();
        assert_eq!(slot.step(2, 7, &[], &label(2)), TagWrite::Label);
        assert_eq!(slot.step(0, 7, &label(2), &[]), TagWrite::Original(&[][..]));
    }

    #[test]
    fn text_the_game_wrote_since_is_not_overwritten_on_restore() {
        let mut slot = TagSlot::new();
        assert_eq!(slot.step(3, 7, &utf16("P1"), &label(3)), TagWrite::Label);
        // The game rewrote the pane: restoring must not clobber it.
        assert_eq!(slot.step(0, 7, &utf16("Luigi"), &[]), TagWrite::Keep);
        assert_eq!(slot.shown(), 0);
        // While showing, a rewrite becomes the new original.
        assert_eq!(slot.step(3, 7, &utf16("P1"), &label(3)), TagWrite::Label);
        assert_eq!(slot.step(4, 7, &utf16("Luigi"), &label(4)), TagWrite::Label);
        assert_eq!(slot.step(0, 7, &label(4), &[]), TagWrite::Original(&utf16("Luigi")[..]));
    }

    #[test]
    fn another_pane_object_is_a_fresh_start() {
        let mut slot = TagSlot::new();
        assert_eq!(slot.step(3, 7, &utf16("P1"), &label(3)), TagWrite::Label);
        // The next match's HUD: a different pane, the game's text on it.
        assert_eq!(slot.step(0, 9, &utf16("P2"), &[]), TagWrite::Keep);
        assert_eq!(slot.shown(), 0);
        assert_eq!(slot.step(3, 7, &utf16("P1"), &label(3)), TagWrite::Label);
        // The label arrives on a different pane: that pane's text is kept.
        assert_eq!(slot.step(3, 9, &utf16("CPU"), &label(3)), TagWrite::Label);
        assert_eq!(slot.step(0, 9, &label(3), &[]), TagWrite::Original(&utf16("CPU")[..]));
        // A forgotten pane (it was not found) is dropped.
        assert_eq!(slot.step(3, 9, &utf16("CPU"), &label(3)), TagWrite::Label);
        slot.forget();
        assert_eq!(slot.step(0, 9, &label(3), &[]), TagWrite::Keep);
    }

    #[test]
    fn a_text_that_cannot_be_kept_is_left_alone() {
        let mut slot = TagSlot::new();
        let long = vec![0x41u16; TAG_CAP + 1];
        assert_eq!(slot.step(3, 7, &long, &label(3)), TagWrite::Keep);
        assert_eq!(slot.shown(), 0);
        let fits = vec![0x41u16; TAG_CAP];
        assert_eq!(slot.step(3, 7, &fits, &label(3)), TagWrite::Label);
        assert_eq!(slot.step(0, 7, &label(3), &[]), TagWrite::Original(&fits[..]));
        // Unknown origin: the label is already there and nothing was kept.
        assert_eq!(slot.step(3, 7, &label(3), &label(3)), TagWrite::Keep);
        assert_eq!(slot.shown(), 0);
        // A label that does not fit is not written.
        assert_eq!(slot.step(3, 7, &utf16("P1"), &[0x41; LABEL_CAP]), TagWrite::Keep);
        assert_eq!(slot.step(3, 7, &utf16("P1"), &[]), TagWrite::Keep);
    }

    #[test]
    fn results_hooks_bind_registers_and_read_dependencies_reject_changes() {
        assert_eq!(RESULTS_INPUT.stock()[0], 0xa947_7bfd); // ldp x29,x30,[sp,#70]
        assert_eq!(RESULTS_CARDS.stock()[0], 0x9001_ab68); // adrp x8,52c5000
        assert_eq!(RESULTS_REMATCH.stock()[0], 0xa940_8660); // ldp x0,x1,[x19,#8]
        for site in SITES.iter().filter(|s| s.kind == Kind::Relied) {
            let at = 0x1000_0000 + site.offset as u64;
            let stock = site.stock();
            assert!(chain::usable(&stock, &stock, at, chain::WORDS));
            for i in 0..chain::WORDS {
                let mut changed = stock;
                changed[i] ^= 1;
                assert!(!chain::usable(&stock, &changed, at, chain::WORDS));
            }
        }
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
