//! The AI's decision as P1's mapped input.
//!
//! The native AI writes its decision into the fighter controller: button
//! bits at `ctrl+0xc`, left stick floats at `+0x10`/`+0x14`. Final input
//! mapping (`main+0x1750dd0`) produces `MappedInputs` (8 bytes, the same
//! button bits); the plugin replaces P1's with this value after the
//! original mapping ran. The right stick stays neutral.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MappedInputs {
    pub buttons: u32,
    pub lstick_x: i8,
    pub lstick_y: i8,
    pub rstick_x: i8,
    pub rstick_y: i8,
}

const _: () = assert!(core::mem::size_of::<MappedInputs>() == 8);

/// A stick float in [-1, 1] as the mapped byte in [-120, 120]; NaN and
/// infinities are neutral.
pub fn axis(value: f32) -> i8 {
    if value.is_finite() {
        (value * 120.0).round().clamp(-120.0, 120.0) as i8
    } else {
        0
    }
}

pub fn from_ai(buttons: u32, stick_x: f32, stick_y: f32) -> MappedInputs {
    MappedInputs {
        buttons,
        lstick_x: axis(stick_x),
        lstick_y: axis(stick_y),
        rstick_x: 0,
        rstick_y: 0,
    }
}

/// One word, so a game-thread writer and the input hook share it atomically.
pub fn pack(input: MappedInputs) -> u64 {
    u64::from(input.buttons)
        | (u64::from(input.lstick_x as u8) << 32)
        | (u64::from(input.lstick_y as u8) << 40)
        | (u64::from(input.rstick_x as u8) << 48)
        | (u64::from(input.rstick_y as u8) << 56)
}

pub fn unpack(word: u64) -> MappedInputs {
    MappedInputs {
        buttons: word as u32,
        lstick_x: (word >> 32) as u8 as i8,
        lstick_y: (word >> 40) as u8 as i8,
        rstick_x: (word >> 48) as u8 as i8,
        rstick_y: (word >> 56) as u8 as i8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_scales_rounds_and_clamps() {
        assert_eq!(axis(1.0), 120);
        assert_eq!(axis(-1.0), -120);
        assert_eq!(axis(0.5), 60);
        assert_eq!(axis(2.0), 120);
        assert_eq!(axis(-3.0), -120);
        assert_eq!(axis(f32::NAN), 0);
        assert_eq!(axis(f32::INFINITY), 0);
    }

    #[test]
    fn pack_round_trips_and_matches_little_endian_layout() {
        let input = from_ai(0x8005, -0.25, 1.0);
        assert_eq!(unpack(pack(input)), input);
        // The packed word is the struct's little-endian memory image.
        let image = u64::from_le_bytes(unsafe { core::mem::transmute::<MappedInputs, [u8; 8]>(input) });
        assert_eq!(pack(input), image);
    }
}
