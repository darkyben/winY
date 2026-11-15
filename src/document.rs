use ropey::Rope;
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

#[derive(Clone)]
struct Revision {
    text: Rope,
    anchor: usize,
    cursor: usize,
    extra: Vec<Range<usize>>,
}

pub struct Document {
    pub text: Rope,
    pub anchor: usize,
    pub cursor: usize,
    pub revision: u64,
    pub extra: Vec<Range<usize>>,
    undo: Vec<Revision>,
    redo: Vec<Revision>,
}

impl Document {
    pub fn tab_title(&self, number: usize) -> String {
        let title: String = self
            .text
            .line(0)
            .chars()
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| !c.is_whitespace())
            .take(48)
            .collect();
        if title.is_empty() {
            number.to_string()
        } else {
            title
        }
    }
    pub fn selected_lines(&self) -> (usize, usize) {
        let selected = self.selection();
        (
            self.text.char_to_line(selected.start),
            self.text.char_to_line(if selected.is_empty() {
                selected.end
            } else {
                selected.end - 1
            }),
        )
    }
    pub fn duplicate(&mut self, whole_lines: bool) {
        let selection = self.selection();
        if !whole_lines && !selection.is_empty() {
            let text = self.text.slice(selection.clone()).to_string();
            self.replace(selection.end..selection.end, &text);
            self.anchor = selection.end;
            return;
        }
        let (first, last) = self.selected_lines();
        let start = self.text.line_to_char(first);
        let end = self.text.line_to_char(last) + self.text.line(last).len_chars();
        let mut text = self.text.slice(start..end).to_string();
        let cursor = self.cursor;
        let anchor = self.anchor;
        let shift = if text.ends_with('\n') {
            text.chars().count()
        } else {
            let newline = if self.text.chars().take(4096).any(|ch| ch == '\r') {
                "\r\n"
            } else {
                "\n"
            };
            text.insert_str(0, newline);
            text.chars().count()
        };
        self.replace(end..end, &text);
        self.cursor = cursor + shift;
        self.anchor = anchor + shift;
    }
    pub fn move_lines(&mut self, down: bool) {
        let (first, last) = self.selected_lines();
        let last_content =
            if self.text.len_chars() > 0 && self.text.char(self.text.len_chars() - 1) == '\n' {
                self.text.len_lines().saturating_sub(2)
            } else {
                self.text.len_lines() - 1
            };
        if (!down && first == 0) || (down && last >= last_content) {
            return;
        }
        let neighbor = if down { last + 1 } else { first - 1 };
        let from_row = first.min(neighbor);
        let to_row = last.max(neighbor);
        let start = self.text.line_to_char(from_row);
        let end = self.text.line_to_char(to_row) + self.text.line(to_row).len_chars();
        let mut bodies = Vec::new();
        let mut endings = Vec::new();
        for row in from_row..=to_row {
            let body_end = self.line_end(row);
            bodies.push(
                self.text
                    .slice(self.text.line_to_char(row)..body_end)
                    .to_string(),
            );
            endings.push(
                self.text
                    .slice(body_end..self.text.line_to_char(row) + self.text.line(row).len_chars())
                    .to_string(),
            );
        }
        let cursor_row = self.text.char_to_line(self.cursor);
        let anchor_row = self.text.char_to_line(self.anchor);
        let cursor_column = self.cursor - self.text.line_to_char(cursor_row);
        let anchor_column = self.anchor - self.text.line_to_char(anchor_row);
        let cursor_at_end = cursor_row == last + 1 && cursor_column == 0;
        let anchor_at_end = anchor_row == last + 1 && anchor_column == 0;
        if down {
            bodies.rotate_right(1);
        } else {
            bodies.rotate_left(1);
        }
        let replacement = bodies
            .into_iter()
            .zip(endings)
            .map(|(body, ending)| body + &ending)
            .collect::<String>();
        self.replace(start..end, &replacement);
        let map = |row: usize| {
            if row >= first && row <= last {
                if down { row + 1 } else { row - 1 }
            } else {
                row
            }
        };
        let cursor_row = map(cursor_row);
        let anchor_row = map(anchor_row);
        self.cursor =
            (self.text.line_to_char(cursor_row) + cursor_column).min(self.line_end(cursor_row));
        self.anchor =
            (self.text.line_to_char(anchor_row) + anchor_column).min(self.text.len_chars());
        let moved_last = if down { last + 1 } else { last - 1 };
        let end_boundary = if moved_last + 1 < self.text.len_lines() {
            self.text.line_to_char(moved_last + 1)
        } else {
            self.text.len_chars()
        };
        if cursor_at_end {
            self.cursor = end_boundary;
        }
        if anchor_at_end {
            self.anchor = end_boundary;
        }
    }
    pub fn transform_case(&mut self, upper: bool) {
        let mut ranges = self.extra.clone();
        ranges.push(self.selection());
        if ranges.iter().all(|range| range.is_empty()) {
            let start = self.boundary(false, true);
            let end = self.boundary(true, true);
            ranges.clear();
            ranges.push(start..end);
        }
        ranges.sort_by_key(|range| range.start);
        ranges.dedup();
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(previous) = merged
                .last_mut()
                .filter(|previous| previous.end > range.start)
            {
                previous.end = previous.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        let ranges = merged;
        let before = self.snapshot();
        let mut delta = 0isize;
        let mut selections = Vec::new();
        let replacements: Vec<_> = ranges
            .iter()
            .map(|range| {
                let text = self.text.slice(range.clone()).to_string();
                if upper {
                    text.to_uppercase()
                } else {
                    text.to_lowercase()
                }
            })
            .collect();
        for (range, text) in ranges.iter().zip(&replacements).rev() {
            self.text.remove(range.clone());
            self.text.insert(range.start, text);
        }
        for (range, text) in ranges.iter().zip(replacements) {
            let start = (range.start as isize + delta) as usize;
            let length = text.chars().count();
            selections.push(start..start + length);
            delta += length as isize - range.len() as isize;
        }
        self.undo.push(before);
        self.redo.clear();
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        if let Some(last) = selections.pop() {
            self.anchor = last.start;
            self.cursor = last.end;
        }
        self.extra = selections;
        self.revision += 1;
    }
    pub fn split_selection(&mut self) {
        let selection = self.selection();
        let (first, last) = self.selected_lines();
        self.extra.clear();
        for row in first..=last {
            let start = self.text.line_to_char(row).max(selection.start);
            let end = self.line_end(row).min(selection.end).max(start);
            if row == last {
                self.anchor = start;
                self.cursor = end;
            } else {
                self.extra.push(start..end);
            }
        }
    }
    pub fn select_all_occurrences(&mut self) {
        if self.selection().is_empty() {
            self.select_next_occurrence();
        }
        let needle = self.text.slice(self.selection()).to_string();
        if needle.is_empty() {
            return;
        }
        let text = self.text.to_string();
        let mut ranges: Vec<_> = text
            .match_indices(&needle)
            .map(|(byte, found)| {
                self.text.byte_to_char(byte)..self.text.byte_to_char(byte + found.len())
            })
            .collect();
        if let Some(last) = ranges.pop() {
            self.anchor = last.start;
            self.cursor = last.end;
            self.extra = ranges;
        }
    }
    pub fn join_lines(&mut self) {
        let (first, last) = self.selected_lines();
        let last = if first == last {
            (last + 1).min(self.text.len_lines() - 1)
        } else {
            last
        };
        if first == last {
            return;
        }
        let start = self.text.line_to_char(first);
        let end = self.line_end(last);
        let text = self.text.slice(start..end).to_string();
        let joined = text
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    line.to_owned()
                } else {
                    format!(" {}", line.trim_start())
                }
            })
            .collect::<String>();
        self.replace(start..end, &joined);
    }
    pub fn match_bracket(&mut self, select: bool) {
        let brackets = [('(', ')'), ('[', ']'), ('{', '}')];
        let mut at = self.cursor;
        if at >= self.text.len_chars()
            || !brackets
                .iter()
                .any(|(a, b)| self.text.char(at) == *a || self.text.char(at) == *b)
        {
            if at == 0 {
                return;
            }
            at -= 1;
        }
        let Some(&(open, close)) = brackets
            .iter()
            .find(|(a, b)| self.text.char(at) == *a || self.text.char(at) == *b)
        else {
            return;
        };
        let forward = self.text.char(at) == open;
        let mut depth = 1usize;
        let mut next = at;
        loop {
            if forward {
                next += 1;
                if next >= self.text.len_chars() {
                    return;
                }
            } else {
                if next == 0 {
                    return;
                }
                next -= 1;
            }
            let character = self.text.char(next);
            if character == if forward { open } else { close } {
                depth += 1;
            }
            if character == if forward { close } else { open } {
                depth -= 1;
            }
            if depth == 0 {
                self.extra.clear();
                if select {
                    self.anchor = at.min(next) + 1;
                    self.cursor = at.max(next);
                } else {
                    self.move_to(next, false);
                }
                return;
            }
        }
    }
    pub fn sort_lines(&mut self, insensitive: bool) {
        let (first, last) = if self.selection().is_empty() {
            (0, self.text.len_lines() - 1)
        } else {
            self.selected_lines()
        };
        let start = self.text.line_to_char(first);
        let end = self.line_end(last);
        let text = self.text.slice(start..end).to_string();
        let mut lines: Vec<_> = text.lines().collect();
        if insensitive {
            lines.sort_by_key(|line| line.to_lowercase());
        } else {
            lines.sort();
        }
        let separator = if text.contains("\r\n") { "\r\n" } else { "\n" };
        self.replace(start..end, &lines.join(separator));
        self.anchor = start;
    }
    pub fn toggle_comment(&mut self, prefix: &str) {
        let (first, last) = self.selected_lines();
        let start = self.text.line_to_char(first);
        let end = self.text.line_to_char(last) + self.text.line(last).len_chars();
        let lines: Vec<_> = (first..=last)
            .map(|row| self.text.line(row).to_string())
            .collect();
        let remove = lines
            .iter()
            .all(|line| line.trim_start().starts_with(prefix));
        let result = lines
            .iter()
            .map(|line| {
                let indentation = line.len() - line.trim_start_matches([' ', '\t']).len();
                if remove {
                    let suffix = &line[indentation + prefix.len()..];
                    format!(
                        "{}{}",
                        &line[..indentation],
                        suffix.strip_prefix(' ').unwrap_or(suffix)
                    )
                } else {
                    format!("{}{prefix} {}", &line[..indentation], &line[indentation..])
                }
            })
            .collect::<String>();
        self.replace(start..end, &result);
        self.anchor = start;
    }
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            anchor: 0,
            cursor: 0,
            revision: 0,
            extra: vec![],
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    fn snapshot(&self) -> Revision {
        Revision {
            text: self.text.clone(),
            anchor: self.anchor,
            cursor: self.cursor,
            extra: self.extra.clone(),
        }
    }
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        if range.is_empty() && text.is_empty() {
            return;
        }
        self.undo.push(self.snapshot());
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.extra.clear();
        self.text.remove(range.clone());
        self.text.insert(range.start, text);
        self.cursor = range.start + text.chars().count();
        self.anchor = self.cursor;
        self.revision += 1;
    }
    pub fn insert(&mut self, text: &str) {
        if self.extra.is_empty() {
            self.replace(self.selection(), text);
            return;
        }
        self.undo.push(self.snapshot());
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        self.redo.clear();
        let mut ranges = self.extra.clone();
        ranges.push(self.selection());
        ranges.sort_by_key(|range| range.start);
        ranges.dedup();
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(previous) = merged
                .last_mut()
                .filter(|previous| previous.end > range.start)
            {
                previous.end = previous.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        let ranges = merged;
        for range in ranges.iter().rev() {
            self.text.remove(range.clone());
            self.text.insert(range.start, text);
        }
        let mut shift = 0isize;
        let mut cursors = Vec::new();
        for range in ranges {
            let cursor = (range.start as isize + shift) as usize + text.chars().count();
            shift += text.chars().count() as isize - range.len() as isize;
            cursors.push(cursor..cursor);
        }
        let last = cursors.pop().expect("primary selection");
        self.cursor = last.end;
        self.anchor = self.cursor;
        self.extra = cursors;
        self.revision += 1;
    }
    pub fn select_next_occurrence(&mut self) {
        let selection = self.selection();
        if selection.is_empty() {
            self.anchor = self.boundary(false, true);
            self.cursor = self.boundary(true, true);
        } else {
            let needle = self.text.slice(selection.clone()).to_string();
            if let Some(range) = self.find(&needle, selection.end, false) {
                if range == selection
                    || self
                        .extra
                        .iter()
                        .any(|r| r.start < range.end && range.start < r.end)
                {
                    return;
                }
                self.extra.push(selection);
                self.anchor = range.start;
                self.cursor = range.end;
            }
        }
    }
    pub fn undo(&mut self, redo: bool) {
        let revision = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(revision) = revision {
            let current = self.snapshot();
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.text = revision.text;
            self.anchor = revision.anchor;
            self.cursor = revision.cursor;
            self.extra = revision.extra;
            self.revision += 1;
        }
    }
    pub fn replace_rope(&mut self, text: Rope) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 256 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.extra.clear();
        self.text = text;
        self.cursor = 0;
        self.anchor = 0;
        self.revision += 1;
    }
    pub fn indent(&mut self, outdent: bool, width: usize) {
        let selected = self.selection();
        let first = self.text.char_to_line(selected.start);
        let last = self.text.char_to_line(if selected.is_empty() {
            selected.end
        } else {
            selected.end - 1
        });
        let start = self.text.line_to_char(first);
        let end = self.text.line_to_char(last) + self.text.line(last).len_chars();
        let mut result = String::new();
        for row in first..=last {
            let line = self.text.line(row);
            if outdent {
                let remove = if line.len_chars() > 0 && line.char(0) == '\t' {
                    1
                } else {
                    line.chars().take(width).take_while(|c| *c == ' ').count()
                };
                result.push_str(&line.slice(remove..).to_string());
            } else {
                result.push_str(&" ".repeat(width));
                result.push_str(&line.to_string());
            }
        }
        self.replace(start..end, &result);
        self.anchor = start;
    }
    pub fn move_to(&mut self, position: usize, extend: bool) {
        self.extra.clear();
        self.cursor = position.min(self.text.len_chars());
        if !extend {
            self.anchor = self.cursor;
        }
    }
    pub fn line_end(&self, line: usize) -> usize {
        let text = self.text.line(line);
        let mut length = text.len_chars();
        while length > 0 && matches!(text.char(length - 1), '\n' | '\r') {
            length -= 1;
        }
        self.text.line_to_char(line) + length
    }
    pub fn boundary(&self, forward: bool, word: bool) -> usize {
        let cursor = self.cursor;
        if word {
            let mut at = cursor;
            let step = |at: usize| if forward { at + 1 } else { at - 1 };
            let current = |at: usize| self.text.char(if forward { at } else { at - 1 });
            let valid = |at| {
                if forward {
                    at < self.text.len_chars()
                } else {
                    at > 0
                }
            };
            if valid(at) {
                let category = |ch: char| {
                    if ch.is_whitespace() {
                        0
                    } else if ch.is_alphanumeric() || ch == '_' {
                        1
                    } else {
                        2
                    }
                };
                let first = category(current(at));
                while valid(at) && category(current(at)) == first {
                    at = step(at);
                }
            }
            return at;
        }
        let byte = self.text.char_to_byte(cursor);
        let mut grapheme = GraphemeCursor::new(byte, self.text.len_bytes(), true);
        let (mut chunk, mut start, _, _) = self.text.chunk_at_byte(byte);
        loop {
            let result = if forward {
                grapheme.next_boundary(chunk, start)
            } else {
                grapheme.prev_boundary(chunk, start)
            };
            match result {
                Ok(Some(byte)) => return self.text.byte_to_char(byte),
                Ok(None) => return cursor,
                Err(GraphemeIncomplete::NextChunk) => {
                    let next = self.text.chunk_at_byte(start + chunk.len());
                    chunk = next.0;
                    start = next.1;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    let previous = self.text.chunk_at_byte(start - 1);
                    chunk = previous.0;
                    start = previous.1;
                }
                Err(GraphemeIncomplete::PreContext(end)) => {
                    let (context, context_start, _, _) = self.text.chunk_at_byte(end - 1);
                    grapheme.provide_context(&context[..end - context_start], context_start);
                }
                Err(GraphemeIncomplete::InvalidOffset) => {
                    unreachable!("rope provided a chunk outside the grapheme cursor")
                }
            }
        }
    }
    pub fn to_utf16(&self, offset: usize) -> usize {
        self.text
            .char_to_utf16_cu(offset.min(self.text.len_chars()))
    }
    pub fn utf16_to_char(&self, offset: usize) -> usize {
        self.text
            .utf16_cu_to_char(offset.min(self.text.len_utf16_cu()))
    }
    pub fn find(&self, needle: &str, from: usize, backwards: bool) -> Option<Range<usize>> {
        if needle.is_empty() {
            return None;
        }
        let text = self.text.to_string();
        let from = self.text.char_to_byte(from.min(self.text.len_chars()));
        let index = if backwards {
            text[..from]
                .rfind(needle)
                .or_else(|| text[from..].rfind(needle).map(|i| from + i))
        } else {
            text[from..]
                .find(needle)
                .map(|i| from + i)
                .or_else(|| text[..from].find(needle))
        }?;
        Some(self.text.byte_to_char(index)..self.text.byte_to_char(index + needle.len()))
    }
    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        let text = self.text.to_string();
        let count = text.matches(needle).count();
        if count > 0 {
            self.replace(0..self.text.len_chars(), &text.replace(needle, replacement));
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_edit_undo_and_utf16() {
        let mut doc = Document::new("a🙂e\u{301}\r\n世界");
        assert_eq!(doc.utf16_to_char(3), 2);
        assert_eq!(doc.to_utf16(2), 3);
        doc.move_to(4, false);
        assert_eq!(doc.boundary(false, false), 2);
        assert_eq!(doc.line_end(0), 4);
        doc.replace(1..4, "é");
        assert_eq!(doc.text.to_string(), "aé\r\n世界");
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "a🙂e\u{301}\r\n世界");
        doc.undo(true);
        assert_eq!(doc.text.to_string(), "aé\r\n世界");
    }
    #[test]
    fn find_wraps_and_replace_is_one_undo() {
        let mut doc = Document::new("été été\nété");
        assert_eq!(doc.find("été", 10, false), Some(0..3));
        assert_eq!(doc.replace_all("été", "🙂"), 3);
        assert_eq!(doc.text.to_string(), "🙂 🙂\n🙂");
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "été été\nété");
    }
    #[test]
    fn large_buffer_edits_preserve_other_lines() {
        let mut doc = Document::new(&"abcdef\n".repeat(100_000));
        doc.replace(350_000..350_006, "xyz");
        assert_eq!(doc.text.len_lines(), 100_001);
        assert_eq!(doc.text.line(50_000).to_string(), "xyz\n");
        doc.undo(false);
        assert_eq!(doc.text.len_chars(), 700_000);
    }
    #[test]
    fn backspace_crosses_crlf_as_one_grapheme() {
        let mut doc = Document::new("one\r\ntwo");
        doc.move_to(5, false);
        assert_eq!(doc.boundary(false, false), 3);
        doc.replace(doc.boundary(false, false)..doc.cursor, "");
        assert_eq!(doc.text.to_string(), "onetwo");
    }
    #[test]
    fn sublime_multiple_occurrences_edit_and_undo_together() {
        let mut doc = Document::new("été été été");
        doc.select_next_occurrence();
        doc.select_next_occurrence();
        doc.select_next_occurrence();
        assert_eq!(doc.extra.len(), 2);
        doc.insert("ok");
        assert_eq!(doc.text.to_string(), "ok ok ok");
        doc.insert("!");
        assert_eq!(doc.text.to_string(), "ok! ok! ok!");
        doc.undo(false);
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "été été été");
    }
    #[test]
    fn grapheme_boundaries_across_rope_chunks() {
        use unicode_segmentation::UnicodeSegmentation;
        let text = format!(
            "{}👨‍👩‍👧‍👦e{}🇫🇷\r\nfin",
            "x".repeat(1_000),
            "\u{301}".repeat(1_200)
        );
        let mut doc = Document::new(&text);
        let mut expected = 0;
        for grapheme in text.graphemes(true) {
            let next = expected + grapheme.chars().count();
            doc.move_to(expected, false);
            assert_eq!(doc.boundary(true, false), next);
            doc.move_to(next, false);
            assert_eq!(doc.boundary(false, false), expected);
            expected = next;
        }
    }
    #[test]
    fn automatic_titles_use_only_first_line() {
        assert_eq!(Document::new("").tab_title(3), "3");
        assert_eq!(
            Document::new("  Bonjour monde\nAutre").tab_title(3),
            "Bonjour"
        );
        assert_eq!(Document::new("\nAutre").tab_title(2), "2");
        assert_eq!(Document::new("été🙂 suite").tab_title(1), "été🙂");
    }
    #[test]
    fn duplicate_last_line_and_selection() {
        let mut doc = Document::new("alpha\r\nbeta");
        doc.move_to(9, false);
        doc.duplicate(true);
        assert_eq!(doc.text.to_string(), "alpha\r\nbeta\r\nbeta");
        assert_eq!(doc.cursor, 15);
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "alpha\r\nbeta");
        doc.anchor = 7;
        doc.cursor = 11;
        doc.duplicate(false);
        assert_eq!(doc.text.to_string(), "alpha\r\nbetabeta");
    }
    #[test]
    fn line_movement_preserves_endings_and_column() {
        let mut doc = Document::new("one\r\ntwo\r\nthree");
        doc.move_to(6, false);
        doc.move_lines(true);
        assert_eq!(doc.text.to_string(), "one\r\nthree\r\ntwo");
        assert_eq!(doc.cursor, 13);
        doc.move_lines(false);
        assert_eq!(doc.text.to_string(), "one\r\ntwo\r\nthree");
        assert_eq!(doc.cursor, 6);
        doc.move_to(0, false);
        let revision = doc.revision;
        doc.move_lines(false);
        assert_eq!(doc.revision, revision);
    }
    #[test]
    fn line_block_selection_moves_including_end_boundary() {
        let mut doc = Document::new("a\nb\nc\nd");
        doc.anchor = 0;
        doc.cursor = 4;
        doc.move_lines(true);
        assert_eq!(doc.text.to_string(), "c\na\nb\nd");
        assert_eq!(doc.selection(), 2..6);
        doc.move_lines(true);
        assert_eq!(doc.text.to_string(), "c\nd\na\nb");
        assert_eq!(doc.selection(), 4..7);
        doc.undo(false);
        assert_eq!(doc.selection(), 2..6);
    }
    #[test]
    fn unicode_case_expansion_and_multiple_ranges_undo() {
        let mut doc = Document::new("straße été");
        doc.extra = vec![0..6];
        doc.anchor = 7;
        doc.cursor = 10;
        doc.transform_case(true);
        assert_eq!(doc.text.to_string(), "STRASSE ÉTÉ");
        assert_eq!(doc.selection(), 8..11);
        assert_eq!(doc.extra, vec![0..7]);
        doc.undo(false);
        assert_eq!(doc.text.to_string(), "straße été");
        doc.transform_case(false);
        assert_eq!(doc.text.to_string(), "straße été");
    }
    #[test]
    fn split_edit_indent_comment_and_join() {
        let mut doc = Document::new("one\ntwo\nthree");
        doc.anchor = 0;
        doc.cursor = 7;
        doc.split_selection();
        doc.insert("x");
        assert_eq!(doc.text.to_string(), "x\nx\nthree");
        doc.undo(false);
        doc.anchor = 0;
        doc.cursor = 7;
        doc.indent(false, 2);
        assert_eq!(doc.text.to_string(), "  one\n  two\nthree");
        doc.toggle_comment("//");
        assert_eq!(doc.text.to_string(), "  // one\n  // two\nthree");
        doc.toggle_comment("//");
        doc.indent(true, 2);
        assert_eq!(doc.text.to_string(), "one\ntwo\nthree");
        doc.move_to(0, false);
        doc.join_lines();
        assert_eq!(doc.text.to_string(), "one two\nthree");
    }
}
