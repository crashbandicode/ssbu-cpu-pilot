//! The HUD mark shown while the CPU drives P1, built without the heap.

/// "CPU Lv<n>" in UTF-16 (the game's pane text encoding) with a trailing
/// NUL; returns the number of code units before the NUL.
pub fn label_utf16(level: u8, buf: &mut [u16; 16]) -> usize {
    let level = crate::settings::clamp_level(level);
    let text = [b'C', b'P', b'U', b' ', b'L', b'v', b'0' + level];
    for (slot, byte) in buf.iter_mut().zip(text) {
        *slot = u16::from(byte);
    }
    buf[text.len()] = 0;
    text.len()
}

/// The same label as ASCII, for logs and an overlay.
pub fn label_ascii(level: u8) -> [u8; 7] {
    [b'C', b'P', b'U', b' ', b'L', b'v', b'0' + crate::settings::clamp_level(level)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label() {
        let mut buf = [0xffffu16; 16];
        let n = label_utf16(3, &mut buf);
        assert_eq!(n, 7);
        assert_eq!(String::from_utf16(&buf[..n]).unwrap(), "CPU Lv3");
        assert_eq!(buf[n], 0);
        assert_eq!(&label_ascii(12), b"CPU Lv9");
    }
}
