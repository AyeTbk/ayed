const ELLIPSIS: &'static str = "…";

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

    pub fn _add_left_aligned(mut self, content: &'a str, data: Data) -> Self {
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

        let maybe_ellipsis_idx = if right_aligned_space < right_aligned_length {
            // Right aligned was "truncated"
            Some(right_aligned_buf_start_idx)
        } else if left_aligned_space < left_aligned_length {
            // Left aligned was "truncated"
            Some(left_aligned_space)
        } else {
            None
        };

        buf.replace_range(
            ..left_aligned_space,
            &left_aligned_content[..left_aligned_space],
        );
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
        let expected = "…nvenu";
        let (result, _payload) = LineBuilder::new_with_length(6)
            .add_right_aligned(content, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__data_payload_indices_are_correct() {
        todo!()
    }
}
