use ropey::Rope;
use std::{collections::HashMap, ops::Range};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct WrapCache {
    pub checkbox_mode: bool,
    key: Option<(u64, usize, usize, bool)>,
    lines: HashMap<usize, Vec<Range<usize>>>,
}
impl WrapCache {
    pub fn line(
        &mut self,
        text: &Rope,
        revision: u64,
        row: usize,
        columns: usize,
        tab: usize,
    ) -> &[Range<usize>] {
        let key = (revision, columns.max(1), tab, self.checkbox_mode);
        if self.key != Some(key) {
            self.lines.clear();
            self.key = Some(key);
        }
        // Bound cache memory when traversing large documents.
        if self.lines.len() > 512 {
            self.lines.clear();
        }
        self.lines.entry(row).or_insert_with(|| {
            segments(
                &text.line(row).to_string(),
                columns.max(1),
                tab,
                self.checkbox_mode,
            )
        })
    }
}
pub fn segments(text: &str, columns: usize, tab: usize, checkboxes: bool) -> Vec<Range<usize>> {
    let text = text.trim_end_matches(['\r', '\n']);
    let mut result = Vec::new();
    let mut start = 0;
    let mut position = 0;
    let mut width = 0;
    let mut boundary = None;
    let mut graphemes = text.grapheme_indices(true);
    while let Some((byte, g)) = graphemes.next() {
        let token = if checkboxes && g == "[" {
            crate::todo::token_len(text, byte)
        } else {
            None
        };
        let length = token.unwrap_or_else(|| g.chars().count());
        if let Some(length) = token {
            for _ in 1..length {
                graphemes.next();
            }
        }
        let cells = if token.is_some() {
            3
        } else if g == "\t" {
            tab
        } else {
            g.width()
        };
        while width + cells > columns && position > start {
            if let Some((word_end, width_at_break)) = boundary.take() {
                result.push(start..word_end);
                start = word_end;
                width -= width_at_break;
            } else {
                result.push(start..position);
                start = position;
                width = 0;
            }
        }
        position += length;
        width += cells;
        if g.chars().all(char::is_whitespace) {
            boundary = Some((position, width));
        }
    }
    result.push(start..position);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkbox_toggle_keeps_wrapped_rows_stable() {
        for columns in 3..18 {
            let rendered: Vec<Vec<String>> =
                ["é [] item words", "é [ ] item words", "é [x] item words"]
                    .iter()
                    .map(|text| {
                        segments(text, columns, 4, true)
                            .into_iter()
                            .map(|range| {
                                text.chars()
                                    .skip(range.start)
                                    .take(range.len())
                                    .collect::<String>()
                                    .replace("[]", "BOX")
                                    .replace("[ ]", "BOX")
                                    .replace("[x]", "BOX")
                            })
                            .collect()
                    })
                    .collect();
            assert_eq!(rendered[0], rendered[1]);
            assert_eq!(rendered[1], rendered[2]);
        }
    }
    #[test]
    fn preserves_text_and_word_boundaries() {
        let text = "hello world abcdefghi";
        let spans = segments(text, 7, 4, false);
        assert_eq!(spans[0], 0..6);
        assert_eq!(
            spans
                .iter()
                .map(|r| text.chars().skip(r.start).take(r.len()).collect::<String>())
                .collect::<String>(),
            text
        );
    }
    #[test]
    fn graphemes_tabs_and_empty_lines() {
        assert_eq!(segments("", 1, 4, false), vec![0..0]);
        assert_eq!(segments("a\u{301}b", 1, 4, false), vec![0..2, 2..3]);
        assert_eq!(segments("\tx", 4, 4, false), vec![0..1, 1..2]);
        assert_eq!(segments("ab\r\n", 1, 4, false), vec![0..1, 1..2]);
    }
}
