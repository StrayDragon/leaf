#[derive(Clone)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub line: usize,
}

pub fn should_hide_single_h1(toc: &[TocEntry]) -> bool {
    let h1_count = toc.iter().filter(|entry| entry.level == 1).count();
    let has_h2 = toc.iter().any(|entry| entry.level == 2);
    h1_count == 1 && has_h2
}

pub fn should_promote_h2_when_no_h1(toc: &[TocEntry]) -> bool {
    !toc.iter().any(|entry| entry.level == 1) && toc.iter().any(|entry| entry.level == 2)
}

pub fn toc_display_level(level: u8, hide_single_h1: bool, promote_h2_root: bool) -> u8 {
    if hide_single_h1 || promote_h2_root {
        match level {
            2 => 1,
            3 => 2,
            _ => level,
        }
    } else {
        level
    }
}

pub fn normalize_toc(mut toc: Vec<TocEntry>) -> Vec<TocEntry> {
    if should_hide_single_h1(&toc) || should_promote_h2_when_no_h1(&toc) {
        toc.retain(|entry| matches!(entry.level, 1..=3));
    } else {
        toc.retain(|entry| matches!(entry.level, 1..=2));
    }
    toc
}
