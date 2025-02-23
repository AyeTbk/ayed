use super::char_index_to_byte_index;

const ELLIPSIS: &'static str = " …";

// TODO rewrite/fix this to be better aware of char boundaries.

pub struct LineBuilder<'a, Data> {
    line_length: usize,
    right_aligned_content: Vec<(&'a str, Data)>,
    left_aligned_content: Vec<(&'a str, Data)>,
}

impl<'a, Data> LineBuilder<'a, Data> {
    pub fn new_with_length(line_length: usize) -> Self {
        Self {
            line_length,
            right_aligned_content: Default::default(),
            left_aligned_content: Default::default(),
        }
    }

    pub fn add_right_aligned(mut self, content: &'a str, data: Data) -> Self {
        self.right_aligned_content.push((content, data));
        self
    }

    pub fn add_left_aligned(mut self, content: &'a str, data: Data) -> Self {
        self.left_aligned_content.push((content, data));
        self
    }

    pub fn build(self) -> (String, Vec<(usize, usize, Data)>) {
        let mut buf = " ".repeat(self.line_length);

        let left_aligned_content = Self::joined_content_string(&self.left_aligned_content);
        let left_aligned_length = left_aligned_content.chars().count();
        let left_aligned_space = left_aligned_length.min(self.line_length);

        let right_aligned_content = Self::joined_content_string(&self.right_aligned_content);
        let right_aligned_length = right_aligned_content.chars().count();
        let right_aligned_space = right_aligned_length.min(self.line_length - left_aligned_space);
        let right_aligned_slice_start_idx = right_aligned_length - right_aligned_space;
        let right_aligned_buf_start_idx = self.line_length - right_aligned_space;

        let maybe_ellipsis_idx =
            if left_aligned_space < left_aligned_length && left_aligned_space > 0 {
                // Left aligned was "truncated"
                Some(left_aligned_space - 1)
            } else if left_aligned_space >= right_aligned_buf_start_idx && left_aligned_space > 0 {
                // Left not truncated but overlaps Right
                Some(left_aligned_space - 1)
            } else if right_aligned_space < right_aligned_length {
                // Right aligned was "truncated"
                Some(right_aligned_buf_start_idx)
            } else {
                None
            };

        // NOTE this is a crappy fix
        let lasb =
            char_index_to_byte_index(&left_aligned_content, left_aligned_space).unwrap_or_default();
        buf.replace_range(..left_aligned_space, &left_aligned_content[..lasb]);
        buf.replace_range(
            right_aligned_buf_start_idx..,
            &right_aligned_content[right_aligned_slice_start_idx..],
        );

        if let Some(ellipsis_idx) = maybe_ellipsis_idx {
            let char_to_replace = buf.chars().nth(ellipsis_idx).unwrap();
            let char_to_replace_byte_size = char_to_replace.len_utf8();
            let ellipsis_end_idx = ellipsis_idx + char_to_replace_byte_size;

            buf.replace_range(ellipsis_idx..ellipsis_end_idx, ELLIPSIS);
        }

        (buf, vec![])
    }

    fn joined_content_string(content: &[(&str, Data)]) -> String {
        content
            .iter()
            .map(|(content, _)| content)
            .fold(String::new(), |mut acc, s| {
                acc.push_str(s);
                acc
            })
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME These tests dont work. The implementation sorta seems to work but I don't like it.
    // Working on Strings is annoying, I should work on Vec<char>s instead, so I can index per character
    // and rely on 1 character == 1 in len.
    // NOTE a Rust char is not a Unicode 'character' and relying on that (which buffer does as of this
    // writing) will lead to incorrect behavior.

    #[test]
    fn build__when_empty__filled_with_spaces() {
        let expected = "                        ";
        let (result, _payload) =
            LineBuilder::<()>::new_with_length(expected.chars().count() as _).build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__right_aligned_is_right_aligned() {
        let expected = "            salut";
        let (result, _payload) = LineBuilder::new_with_length(expected.chars().count() as _)
            .add_right_aligned("salut", ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_not_enough_space__right_aligned_is_ellipsized() {
        let content = "bienvenu";
        let expected = " …venu";
        let (result, _payload) = LineBuilder::new_with_length(6)
            .add_right_aligned(content, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_not_enough_space__left_aligned_is_ellipsized() {
        let content = "bienvenu";
        let expected = "bien …";
        let (result, _payload) = LineBuilder::new_with_length(6)
            .add_left_aligned(content, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_left_aligned_is_ellipsized_and_completely_overlaps_right_aligned__dont_crash_plz()
     {
        let lcontent = "bienvenu";
        let rcontent = "allo";
        let expected = "bi …";
        let (result, _payload) = LineBuilder::new_with_length(4)
            .add_left_aligned(lcontent, ())
            .add_right_aligned(rcontent, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_left_aligned_has_just_enough_space_but_right_aligned_is_ellipsized_so_the_last_character_needs_to_be_ellipsis__dont_crash_plz()
     {
        let expected = ":edit the file plz tyvm rlly appreciated like i mean it dud …";
        let (result, _payload) = LineBuilder::new_with_length(61)
            .add_left_aligned(":", ())
            .add_left_aligned(
                "edit the file plz tyvm rlly appreciated like i mean it dude",
                (),
            )
            .add_left_aligned(" ", ())
            .add_right_aligned("Cargo.toml", ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__data_payload_indices_are_correct() {
        todo!()
    }
}
