pub mod line_builder;

pub fn line_clamped_filled(line: &str, start: u32, char_count: u32, fill: char) -> String {
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

pub fn char_index_to_byte_index(s: &str, ch_idx: u32) -> Option<usize> {
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

pub fn char_index_to_byte_index_end(s: &str, ch_idx: u32) -> Option<usize> {
    s.char_indices()
        .chain(Some((s.len(), '\n')))
        .chain(Some((s.len() + 1, '\0')))
        .skip(ch_idx as _)
        .skip(1)
        .map(|(idx, _)| idx)
        .next()
}

pub fn byte_index_to_char_index(s: &str, byte_idx: usize) -> Option<u32> {
    if byte_idx == 0 {
        Some(0)
    } else {
        let mut ch_idx = 0;
        let mut found_it = false;
        for (i, (idx, _)) in s.char_indices().chain(Some((s.len(), '\n'))).enumerate() {
            if idx > byte_idx {
                found_it = true;
                break;
            }
            ch_idx = i as u32;
        }
        found_it.then_some(ch_idx)
    }
}

pub fn char_count(s: &str) -> u32 {
    s.chars().count().try_into().unwrap()
}
