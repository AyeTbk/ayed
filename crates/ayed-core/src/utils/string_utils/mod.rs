pub mod grid_string_builder;
pub mod line_builder;
pub mod ops;

pub fn line_clamped_filled(line: &str, start: usize, char_count: usize, fill: char) -> String {
    let mut s = String::new();
    let mut char_taken_count = 0;
    for ch in line.chars().skip(start as _).take(char_count as _) {
        s.push(ch);
        char_taken_count += 1;
    }
    let missing_char_count = char_count.saturating_sub(char_taken_count);
    for _ in 0..missing_char_count {
        s.push(fill);
    }
    s
}

pub fn char_index_to_byte_index(s: &str, ch_idx: usize) -> Option<usize> {
    if ch_idx == 0 {
        Some(0)
    } else {
        s.char_indices()
            .chain(Some((s.len(), '\n')))
            .skip(ch_idx as _)
            .map(|(idx, _)| idx)
            .next()
    }
}

#[expect(dead_code)]
pub fn char_index_to_byte_index_end(s: &str, ch_idx: usize) -> Option<usize> {
    s.char_indices()
        .chain(Some((s.len(), '\n')))
        .chain(Some((s.len() + 1, '\0')))
        .skip(ch_idx as _)
        .skip(1)
        .map(|(idx, _)| idx)
        .next()
}

pub fn byte_index_to_char_index(s: &str, byte_idx: usize) -> Option<usize> {
    if byte_idx == 0 {
        Some(0)
    } else {
        let mut ch_idx = 0;
        let mut found_it = false;
        for (i, (idx, _)) in s.char_indices().chain(Some((s.len(), '\n'))).enumerate() {
            ch_idx = i;
            if idx >= byte_idx {
                found_it = true;
                break;
            }
        }
        found_it.then_some(ch_idx)
    }
}

pub fn char_count(s: &str) -> usize {
    s.chars().count()
}
