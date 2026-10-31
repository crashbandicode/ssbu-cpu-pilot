//! CPU level (1..=9) to native AI rank.
//!
//! 13.0.5 keeps the ranks of levels 2..=9 in a table at `main+0x4470dc0`;
//! level 1 is rank 0 (the Training Modpack fork measured both ends, rank 0
//! and 100, against the native CPU at levels 1 and 9). A table that does
//! not read exactly this way refuses every level.
pub const RANK_TABLE_13_0_5: [i32; 8] = [15, 21, 31, 42, 48, 60, 75, 100];

/// The nine ranks, or `None` if `table` is not the 13.0.5 table.
pub fn ranks(table: [i32; 8]) -> Option<[u8; 9]> {
    if table != RANK_TABLE_13_0_5 {
        return None;
    }
    let mut out = [0u8; 9];
    for (slot, value) in out[1..].iter_mut().zip(table) {
        *slot = value as u8;
    }
    Some(out)
}

pub fn rank_for(level: u8, table: [i32; 8]) -> Option<u8> {
    if !(crate::settings::LEVEL_MIN..=crate::settings::LEVEL_MAX).contains(&level) {
        return None;
    }
    ranks(table).map(|r| r[usize::from(level - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder() {
        assert_eq!(
            ranks(RANK_TABLE_13_0_5),
            Some([0, 15, 21, 31, 42, 48, 60, 75, 100])
        );
        assert_eq!(rank_for(1, RANK_TABLE_13_0_5), Some(0));
        assert_eq!(rank_for(3, RANK_TABLE_13_0_5), Some(21));
        assert_eq!(rank_for(9, RANK_TABLE_13_0_5), Some(100));
    }

    #[test]
    fn refuses_other_tables_and_levels() {
        let mut table = RANK_TABLE_13_0_5;
        table[7] = 99;
        assert_eq!(ranks(table), None);
        assert_eq!(rank_for(0, RANK_TABLE_13_0_5), None);
        assert_eq!(rank_for(10, RANK_TABLE_13_0_5), None);
    }
}
