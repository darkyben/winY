use crate::document::Document;
use ropey::RopeSlice;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub range: Range<usize>,
    pub checked: bool,
}
impl Marker {
    // Keep the same shaped advance for [] and [ ]/[x], without changing the source.
    pub fn display_spaces(&self, position: usize) -> usize {
        if position == self.range.start {
            4 - self.range.len()
        } else {
            1
        }
    }
}
/// Recognize a standalone ASCII task marker at a UTF-8 byte boundary.
pub fn token_len(text: &str, byte: usize) -> Option<usize> {
    if byte > 0 && !text[..byte].chars().next_back()?.is_whitespace() {
        return None;
    }
    let tail = &text[byte..];
    let length = if tail.starts_with("[]") {
        2
    } else if ["[ ]", "[x]", "[X]"].iter().any(|s| tail.starts_with(s)) {
        3
    } else {
        return None;
    };
    if tail[length..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace())
    {
        return None;
    }
    Some(length)
}
// Scan only the displayed slice, with a small lookaround for token boundaries.
pub fn markers(text: RopeSlice<'_>, visible: Range<usize>) -> Vec<Marker> {
    let begin = visible.start.saturating_sub(1);
    let finish = (visible.end + 3).min(text.len_chars());
    let chars: Vec<_> = text.slice(begin..finish).chars().collect();
    let mut result = Vec::new();
    for pos in visible.start..visible.end {
        let i = pos - begin;
        if chars[i] != '[' || (pos > 0 && !chars[i - 1].is_whitespace()) {
            continue;
        }
        let (length, checked) = match (chars.get(i + 1), chars.get(i + 2)) {
            (Some(']'), _) => (2, false),
            (Some(' '), Some(']')) => (3, false),
            (Some('x' | 'X'), Some(']')) => (3, true),
            _ => continue,
        };
        let end = pos + length;
        if end > visible.end || chars.get(i + length).is_some_and(|c| !c.is_whitespace()) {
            continue;
        }
        result.push(Marker {
            range: pos..end,
            checked,
        });
    }
    result
}
impl Document {
    pub fn checkbox_at(&self, position: usize) -> Option<Range<usize>> {
        markers(
            self.text.slice(..),
            position.saturating_sub(3)..(position + 3).min(self.text.len_chars()),
        )
        .into_iter()
        .find(|m| m.range.contains(&position))
        .map(|m| m.range)
    }
    pub fn selected_checkbox(&self) -> Option<Range<usize>> {
        let selection = self.selection();
        if !self.extra.is_empty() || selection.is_empty() {
            return None;
        }
        self.checkbox_at(selection.start)
            .filter(|r| *r == selection)
    }
    pub fn select_checkbox_at_cursor(&mut self) {
        if self.selection().is_empty()
            && self.extra.is_empty()
            && let Some(range) = self.checkbox_at(self.cursor)
        {
            self.anchor = range.start;
            self.cursor = range.end;
        }
    }
    pub fn move_checkbox(&mut self, down: bool) -> bool {
        let Some(current) = self.selected_checkbox() else {
            return false;
        };
        let row = self.text.char_to_line(current.start);
        let next = if down {
            row + 1
        } else {
            let Some(row) = row.checked_sub(1) else {
                return true;
            };
            row
        };
        if next >= self.text.len_lines() {
            return true;
        }
        let column = current.start - self.text.line_to_char(row);
        let line = self.text.line(next);
        let candidate = markers(line, 0..line.len_chars())
            .into_iter()
            .min_by_key(|m| m.range.start.abs_diff(column));
        let Some(candidate) = candidate else {
            return false;
        };
        let base = self.text.line_to_char(next);
        self.anchor = base + candidate.range.start;
        self.cursor = base + candidate.range.end;
        true
    }
    pub fn toggle_checkbox(&mut self, range: Range<usize>) -> bool {
        if range.end > self.text.len_chars() || range.start >= range.end {
            return false;
        }
        let replacement = match self.text.slice(range.clone()).to_string().as_str() {
            "[]" | "[ ]" => "[x]",
            "[x]" | "[X]" => "[ ]",
            _ => return false,
        };
        let adjust = |p: usize| {
            if p >= range.end {
                p - range.len() + replacement.len()
            } else if p > range.start {
                range.start + (p - range.start).min(replacement.len())
            } else {
                p
            }
        };
        let anchor = adjust(self.anchor);
        let cursor = adjust(self.cursor);
        let extra = self
            .extra
            .iter()
            .map(|r| adjust(r.start)..adjust(r.end))
            .collect();
        self.replace(range, replacement);
        self.anchor = anchor;
        self.cursor = cursor;
        self.extra = extra;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::{Document, markers};
    use ropey::Rope;
    #[test]
    fn keyboard_checkbox_list_preserves_selection_after_toggle() {
        let mut doc = Document::new("[] first\n  [ ] second\n[x] third");
        doc.cursor = 1;
        doc.anchor = 1;
        doc.select_checkbox_at_cursor();
        assert_eq!(doc.selection(), 0..2);
        assert!(doc.toggle_checkbox(doc.selected_checkbox().unwrap()));
        assert_eq!(doc.selection(), 0..3);
        assert!(doc.move_checkbox(true));
        assert_eq!(doc.text.slice(doc.selection()).to_string(), "[ ]");
        assert!(doc.toggle_checkbox(doc.selected_checkbox().unwrap()));
        assert!(doc.move_checkbox(true));
        assert!(doc.toggle_checkbox(doc.selected_checkbox().unwrap()));
        assert_eq!(doc.text.to_string(), "[x] first\n  [x] second\n[ ] third");
        assert!(doc.move_checkbox(false));
        assert_eq!(doc.text.slice(doc.selection()).to_string(), "[x]");
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "[x] first\n  [x] second\n[x] third");
    }
    #[test]
    fn all_states_have_identical_display_width() {
        for text in ["[]", "[ ]", "[x]", "[X]"] {
            let rope = Rope::from_str(text);
            let marker = markers(rope.slice(..), 0..rope.len_chars()).remove(0);
            assert_eq!(
                marker
                    .range
                    .clone()
                    .map(|i| marker.display_spaces(i))
                    .sum::<usize>(),
                3
            );
        }
    }
    #[test]
    fn detects_plain_and_markdown_tasks_without_code_or_links() {
        let text =
            Rope::from_str("[] first\n- [ ] second\n[x] done\n[X] done\narray[x] [x](url) [[]]\n");
        let found = markers(text.slice(..), 0..text.len_chars());
        assert_eq!(found.len(), 4);
        assert!(!found[0].checked);
        assert!(found[2].checked);
    }
    #[test]
    fn visible_bounds_and_unicode_use_character_offsets() {
        let text = Rope::from_str("é🙂 [] task");
        assert_eq!(markers(text.slice(..), 3..5)[0].range, 3..5);
        assert!(markers(text.slice(..), 4..7).is_empty());
        assert!(markers(text.slice(..), 0..4).is_empty());
    }
    #[test]
    fn toggle_preserves_carets_and_is_undoable() {
        let mut doc = Document::new("[] é🙂 task\r\n");
        doc.cursor = doc.text.len_chars();
        doc.anchor = doc.cursor;
        doc.extra = vec![5..7];
        let old_cursor = doc.cursor;
        assert!(doc.toggle_checkbox(0..2));
        assert_eq!(doc.text.to_string(), "[x] é🙂 task\r\n");
        assert_eq!(doc.cursor, old_cursor + 1);
        assert_eq!(doc.extra, vec![6..8]);
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "[] é🙂 task\r\n");
        assert_eq!(doc.cursor, old_cursor);
        doc.undo(true);
        assert!(doc.toggle_checkbox(0..3));
        assert!(doc.text.to_string().starts_with("[ ]"));
        assert!(!doc.toggle_checkbox(4..5));
    }
}
