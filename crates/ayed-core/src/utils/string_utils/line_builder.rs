pub struct LineBuilder<'a, Data> {
    line_length: usize,
    right_aligned_content: Vec<(&'a str, Data)>,
    left_aligned_content: Vec<(&'a str, Data)>,
}

impl<'a, Data> LineBuilder<'a, Data> {
    pub fn new() -> Self {
        Self {
            line_length: 1,
            right_aligned_content: Default::default(),
            left_aligned_content: Default::default(),
        }
    }

    pub fn with_length(mut self, line_length: usize) -> Self {
        self.line_length = line_length;
        self
    }

    pub fn add_right_aligned(mut self, content: &'a str, data: Data) -> Self {
        self.right_aligned_content.push((content, data));
        self
    }

    pub fn add_left_aligned(mut self, content: &'a str, data: Data) -> Self {
        self.left_aligned_content.push((content, data));
        self
    }

    pub fn build(self) -> (String, Vec<PositionedData<Data>>) {
        let ellipsis_len = 2; // ellipsis + a space

        let mut datas = Vec::new();
        let mut buf = String::new();

        // Build left-aligned content
        let mut remaining_len = self.line_length as i32;
        let mut left_ellipsize = false;
        let mut char_count = 0;
        for (content, data) in self.left_aligned_content.into_iter() {
            let data_start_byte_idx = buf.len();
            let data_start_char_idx = char_count;
            for ch in content.chars() {
                if remaining_len == ellipsis_len {
                    left_ellipsize = true;
                    break;
                }
                buf.push(ch);
                remaining_len -= 1;
                char_count += 1;
            }
            let data_end_byte_idx = buf.len();
            let data_end_char_idx = char_count;
            datas.push(PositionedData {
                bytes: (data_start_byte_idx, data_end_byte_idx),
                chars: (data_start_char_idx, data_end_char_idx),
                data,
            });
            if left_ellipsize {
                break;
            }
        }
        if left_ellipsize {
            buf.push_str("… ");
            remaining_len -= 2;
        }

        // Build right aligned content, in reverse
        if remaining_len > ellipsis_len {
            let mut rdatas: Vec<PositionedData<Data>> = Vec::new();
            let mut rbuf = String::new();
            let mut right_ellipsize = false;
            let mut rchar_count = 0;
            for (content, data) in self.right_aligned_content.into_iter().rev() {
                let data_start_byte_idx = rbuf.len();
                let data_start_char_idx = rchar_count;
                for ch in content.chars().rev() {
                    if remaining_len == ellipsis_len {
                        right_ellipsize = true;
                        break;
                    }
                    rbuf.push(ch);
                    remaining_len -= 1;
                    rchar_count += 1;
                }
                let data_end_byte_idx = rbuf.len();
                let data_end_char_idx = rchar_count;
                rdatas.push(PositionedData {
                    bytes: (data_start_byte_idx, data_end_byte_idx),
                    chars: (data_start_char_idx, data_end_char_idx),
                    data,
                });
                if right_ellipsize {
                    break;
                }
            }
            if right_ellipsize {
                rbuf.push_str("… ");
                remaining_len -= 2;
            }

            // Optional padding
            if remaining_len > 0 {
                for _ in 0..remaining_len {
                    buf.push(' ');
                }
            }

            // Fixup rdatas indices
            let full_buf_byte_len = buf.len() + rbuf.len();
            let full_buf_char_len = self.line_length;
            for data in &mut rdatas {
                data.bytes.0 = full_buf_byte_len - data.bytes.0;
                data.bytes.1 = full_buf_byte_len - data.bytes.1;

                data.chars.0 = full_buf_char_len - data.chars.0;
                data.chars.1 = full_buf_char_len - data.chars.1;
            }

            // Combine left and right
            for ch in rbuf.chars().rev() {
                buf.push(ch);
            }
            datas.extend(rdatas);
        }

        (buf, datas)
    }
}

#[derive(Debug, Clone)]
pub struct PositionedData<Data> {
    pub bytes: (usize, usize),
    pub chars: (usize, usize),
    pub data: Data,
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::utils::string_utils::char_count;

    use super::*;

    // FIXME These tests dont work. The implementation sorta seems to work but I don't like it.
    // Working on Strings is annoying, I should work on Vec<char>s instead, so I can index per character
    // and rely on 1 character == 1 in len.
    // NOTE a Rust char is not a Unicode 'character' and relying on that (which buffer does as of this
    // writing) will lead to incorrect behavior.

    #[test]
    fn build__when_empty__filled_with_spaces() {
        let expected = "                        ";
        let (result, _payload) = LineBuilder::<()>::new()
            .with_length(char_count(expected) as _)
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__right_aligned_is_right_aligned() {
        let expected = "            salut";
        let (result, _payload) = LineBuilder::new()
            .with_length(char_count(expected) as _)
            .add_right_aligned("salut", ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_not_enough_space__right_aligned_is_ellipsized() {
        let content = "bienvenu";
        let expected = " …venu";
        let (result, _payload) = LineBuilder::new()
            .with_length(6)
            .add_right_aligned(content, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_not_enough_space__left_aligned_is_ellipsized() {
        let content = "bienvenu";
        let expected = "bien… ";
        let (result, _payload) = LineBuilder::new()
            .with_length(6)
            .add_left_aligned(content, ())
            .build();

        assert_eq!(result, expected);
    }

    #[test]
    fn build__when_left_aligned_is_ellipsized_and_completely_overlaps_right_aligned__dont_crash_plz()
     {
        let lcontent = "bienvenu";
        let rcontent = "allo";
        let expected = "bi… ";
        let (result, _payload) = LineBuilder::new()
            .with_length(4)
            .add_left_aligned(lcontent, ())
            .add_right_aligned(rcontent, ())
            .build();

        assert_eq!(result, expected);
    }
}
