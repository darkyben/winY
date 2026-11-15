use crate::appearance::Appearance;
use crate::document::Document;
use gpui::{prelude::*, *};
use std::{
    ops::Range,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use syntect::{easy::HighlightLines, parsing::SyntaxSet};

type Colors = Vec<Vec<(usize, u32)>>;
static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();

pub struct Editor {
    pub document: Document,
    pub focus: FocusHandle,
    pub scroll: usize,
    pub horizontal: usize,
    pub syntax: String,
    pub single_line: bool,
    pub appearance: Appearance,
    carets: crate::caret::Carets,
    wrap: crate::wrap::WrapCache,
    wrap_offset: usize,
    chord: bool,
    rectangle: Option<(Point<Pixels>, Vec<Range<usize>>)>,
    bounds: Bounds<Pixels>,
    lines: Vec<VisualLine>,
    checkboxes: Vec<(Bounds<Pixels>, Range<usize>, u64)>,
    selecting: bool,
    marked: Option<Range<usize>>,
    preferred_column: Option<usize>,
    colors: Colors,
    color_key: Option<(u64, usize, usize, String, String)>,
    color_start: usize,
    generation: Arc<AtomicU64>,
}
struct VisualLine {
    shape: ShapedLine,
    origin: Point<Pixels>,
    offsets: Vec<(usize, usize)>,
    start: usize,
    end: usize,
}
impl VisualLine {
    fn x(&self, position: usize) -> Pixels {
        let index = self
            .offsets
            .iter()
            .find(|(_, offset)| *offset >= position)
            .map(|(byte, _)| *byte)
            .unwrap_or(self.shape.text.len());
        self.origin.x + self.shape.x_for_index(index)
    }
    fn offset(&self, x: Pixels) -> usize {
        let index = self.shape.closest_index_for_x(x - self.origin.x);
        self.offsets
            .iter()
            .rev()
            .find(|(byte, _)| *byte <= index)
            .map(|(_, position)| *position)
            .unwrap_or(self.start)
    }
}

impl Editor {
    pub fn new(text: &str, single_line: bool, cx: &mut Context<Self>) -> Self {
        Self {
            document: Document::new(text),
            focus: cx.focus_handle(),
            scroll: 0,
            horizontal: 0,
            syntax: "txt".into(),
            single_line,
            appearance: cx.global::<Appearance>().clone(),
            carets: Default::default(),
            wrap: Default::default(),
            wrap_offset: 0,
            chord: false,
            rectangle: None,
            bounds: Bounds::default(),
            lines: vec![],
            checkboxes: vec![],
            selecting: false,
            marked: None,
            preferred_column: None,
            colors: vec![],
            color_key: None,
            color_start: 0,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }
    fn has_checkboxes(&self) -> bool {
        !self.single_line && matches!(self.syntax.as_str(), "txt" | "md" | "markdown")
    }
    pub fn changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal();
        window.notify_text_input_changed(&self.focus);
        cx.notify();
    }
    pub fn reveal(&mut self) {
        let row = self.document.text.char_to_line(self.document.cursor);
        let column = self.document.cursor - self.document.text.line_to_char(row);
        let rows =
            ((f32::from(self.bounds.size.height) / self.appearance.line_height()) as usize).max(1);
        let columns = ((f32::from(self.bounds.size.width) / self.appearance.cell_width()) as usize)
            .saturating_sub(2)
            .max(1);
        if self.wrapping() {
            self.horizontal = 0;
            let spans = self.wrap.line(
                &self.document.text,
                self.document.revision,
                row,
                columns,
                self.appearance.tab_size,
            );
            let segment = spans
                .iter()
                .rposition(|span| span.start <= column)
                .unwrap_or(0);
            if row < self.scroll || (row == self.scroll && segment < self.wrap_offset) {
                self.scroll = row;
                self.wrap_offset = segment;
            } else {
                let visible = self.visible_spans(rows, columns);
                if !visible.iter().any(|(r, span)| {
                    *r == row
                        && column >= span.start
                        && (column < span.end
                            || span.end
                                == self.document.line_end(row)
                                    - self.document.text.line_to_char(row))
                }) {
                    self.scroll = row;
                    self.wrap_offset = segment.saturating_sub(rows.saturating_sub(1));
                }
            }
            return;
        }
        if row < self.scroll {
            self.scroll = row;
        }
        if row >= self.scroll + rows {
            self.scroll = row + 1 - rows;
        }
        if column < self.horizontal {
            self.horizontal = column;
        }
        if column >= self.horizontal + columns {
            self.horizontal = column + 1 - columns;
        }
    }
    fn wrapping(&self) -> bool {
        self.appearance.word_wrap && !self.single_line
    }
    fn wrap_columns(&self) -> usize {
        ((f32::from(self.bounds.size.width) / self.appearance.cell_width()) as usize)
            .saturating_sub(2)
            .max(1)
    }
    fn visible_spans(&mut self, count: usize, columns: usize) -> Vec<(usize, Range<usize>)> {
        let mut result = Vec::with_capacity(count);
        for row in self.scroll..self.document.text.len_lines() {
            let spans = self.wrap.line(
                &self.document.text,
                self.document.revision,
                row,
                columns,
                self.appearance.tab_size,
            );
            let skip = if row == self.scroll {
                self.wrap_offset.min(spans.len() - 1)
            } else {
                0
            };
            result.extend(
                spans
                    .iter()
                    .skip(skip)
                    .take(count - result.len())
                    .cloned()
                    .map(|span| (row, span)),
            );
            if result.len() == count {
                break;
            }
        }
        result
    }
    fn scroll_wrapped(&mut self, delta: isize) {
        let columns = self.wrap_columns();
        for _ in 0..delta.unsigned_abs().min(1000) {
            if delta > 0 {
                let count = self
                    .wrap
                    .line(
                        &self.document.text,
                        self.document.revision,
                        self.scroll,
                        columns,
                        self.appearance.tab_size,
                    )
                    .len();
                if self.wrap_offset + 1 < count {
                    self.wrap_offset += 1;
                } else if self.scroll + 1 < self.document.text.len_lines() {
                    self.scroll += 1;
                    self.wrap_offset = 0;
                }
            } else if self.wrap_offset > 0 {
                self.wrap_offset -= 1;
            } else if self.scroll > 0 {
                self.scroll -= 1;
                self.wrap_offset = self
                    .wrap
                    .line(
                        &self.document.text,
                        self.document.revision,
                        self.scroll,
                        columns,
                        self.appearance.tab_size,
                    )
                    .len()
                    - 1;
            }
        }
    }
    fn hit(&self, point: Point<Pixels>) -> usize {
        let row = ((f32::from(point.y - self.bounds.top()).max(0.) / self.appearance.line_height())
            as usize)
            .min(self.lines.len().saturating_sub(1));
        self.lines
            .get(row)
            .map(|line| line.offset(point.x))
            .unwrap_or(0)
    }
    fn rectangular_selection(&mut self, point: Point<Pixels>) {
        let Some((origin, previous)) = self.rectangle.as_ref() else {
            return;
        };
        let start_row = ((f32::from(origin.y - self.bounds.top()).max(0.)
            / self.appearance.line_height()) as usize)
            .min(self.lines.len().saturating_sub(1));
        let end_row = ((f32::from(point.y - self.bounds.top()).max(0.)
            / self.appearance.line_height()) as usize)
            .min(self.lines.len().saturating_sub(1));
        let mut selections = previous.clone();
        for row in start_row.min(end_row)..=start_row.max(end_row) {
            if let Some(line) = self.lines.get(row) {
                let from = line.offset(origin.x);
                let to = line.offset(point.x);
                selections.push(from.min(to)..from.max(to));
            }
        }
        if let Some(last) = selections.pop() {
            self.document.anchor = last.start;
            self.document.cursor = last.end;
        }
        self.document.extra = selections;
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let control = modifiers.control;
        let shift = modifiers.shift;
        if self.has_checkboxes()
            && !control
            && !shift
            && !modifiers.alt
            && let Some(range) = self.document.selected_checkbox()
        {
            let handled = match key {
                "space" | " " => self.document.toggle_checkbox(range),
                "up" | "down" => {
                    self.preferred_column = Some(
                        range.start
                            - self
                                .document
                                .text
                                .line_to_char(self.document.text.char_to_line(range.start)),
                    );
                    self.document.move_checkbox(key == "down")
                }
                "left" => {
                    self.document.move_to(range.start.saturating_sub(1), false);
                    true
                }
                "right" => {
                    self.document.move_to(range.end, false);
                    true
                }
                _ => false,
            };
            if handled {
                self.preferred_column = None;
                self.marked = None;
                self.changed(window, cx);
                window.prevent_default();
                cx.stop_propagation();
                return;
            }
        }
        if self.chord {
            self.chord = false;
            if control && matches!(key, "u" | "l" | "k" | "d") {
                match key {
                    "u" => self.document.transform_case(true),
                    "l" => self.document.transform_case(false),
                    "k" => {
                        let end = self
                            .document
                            .line_end(self.document.text.char_to_line(self.document.cursor));
                        self.document.replace(self.document.cursor..end, "");
                    }
                    "d" => self.document.select_next_occurrence(),
                    _ => {}
                }
                self.changed(window, cx);
                cx.stop_propagation();
                return;
            }
        }
        if control && key == "k" && !shift && !self.single_line {
            self.chord = true;
            cx.stop_propagation();
            return;
        }
        if self.single_line
            && (matches!(key, "up" | "down" | "enter" | "escape" | "tab")
                || (control && shift && key == "p"))
        {
            return;
        }
        if self.wrapping() && !control && matches!(key, "up" | "down" | "pageup" | "pagedown") {
            let columns = self.wrap_columns();
            let mut row = self.document.text.char_to_line(self.document.cursor);
            let offset = self.document.cursor - self.document.text.line_to_char(row);
            let spans = self.wrap.line(
                &self.document.text,
                self.document.revision,
                row,
                columns,
                self.appearance.tab_size,
            );
            let mut segment = spans.iter().rposition(|r| r.start <= offset).unwrap_or(0);
            let column = *self
                .preferred_column
                .get_or_insert(offset - spans[segment].start);
            let distance = if key.starts_with("page") {
                (f32::from(self.bounds.size.height) / self.appearance.line_height()).max(1.)
                    as usize
            } else {
                1
            };
            for _ in 0..distance {
                if key == "up" || key == "pageup" {
                    if segment > 0 {
                        segment -= 1;
                    } else if row > 0 {
                        row -= 1;
                        segment = self
                            .wrap
                            .line(
                                &self.document.text,
                                self.document.revision,
                                row,
                                columns,
                                self.appearance.tab_size,
                            )
                            .len()
                            - 1;
                    }
                } else {
                    let count = self
                        .wrap
                        .line(
                            &self.document.text,
                            self.document.revision,
                            row,
                            columns,
                            self.appearance.tab_size,
                        )
                        .len();
                    if segment + 1 < count {
                        segment += 1;
                    } else if row + 1 < self.document.text.len_lines() {
                        row += 1;
                        segment = 0;
                    }
                }
            }
            let spans = self.wrap.line(
                &self.document.text,
                self.document.revision,
                row,
                columns,
                self.appearance.tab_size,
            );
            let span = &spans[segment];
            let offset = (span.start + column).min(span.end);
            self.document
                .move_to(self.document.text.line_to_char(row) + offset, shift);
            if self.has_checkboxes() && !shift {
                self.document.select_checkbox_at_cursor();
            }
            self.changed(window, cx);
            cx.stop_propagation();
            return;
        }
        let doc = &mut self.document;
        let selection = doc.selection();
        let mut vertical = false;
        match (control, key) {
            (true, "m") if !self.single_line => doc.match_bracket(shift),
            (true, "u") if !self.single_line => {
                if let Some(range) = doc.extra.pop() {
                    doc.anchor = range.start;
                    doc.cursor = range.end;
                } else {
                    doc.undo(false);
                }
            }
            (true, "up" | "down") if shift && !self.single_line => doc.move_lines(key == "down"),
            (true, "up" | "down") if modifiers.alt && !self.single_line => {
                let row = doc.text.char_to_line(doc.cursor);
                let column = doc.cursor - doc.text.line_to_char(row);
                let next = if key == "up" {
                    row.saturating_sub(1)
                } else {
                    (row + 1).min(doc.text.len_lines() - 1)
                };
                if next != row {
                    let old = doc.selection();
                    let at = (doc.text.line_to_char(next) + column).min(doc.line_end(next));
                    doc.extra.push(old);
                    doc.anchor = at;
                    doc.cursor = at;
                }
            }
            (true, "up" | "down") if !self.single_line => {
                self.scroll = if key == "up" {
                    self.scroll.saturating_sub(1)
                } else {
                    (self.scroll + 1).min(doc.text.len_lines() - 1)
                };
                cx.notify();
                cx.stop_propagation();
                return;
            }
            (false, "f3") if modifiers.alt && !self.single_line => doc.select_all_occurrences(),
            (_, "f9") if !self.single_line => doc.sort_lines(!shift),
            (true, "j") if !self.single_line => doc.join_lines(),
            (true, "/") if !self.single_line => doc.toggle_comment(
                if ["py", "sh", "yaml", "yml", "toml", "rb"].contains(&self.syntax.as_str()) {
                    "#"
                } else {
                    "//"
                },
            ),
            (true, "a") => {
                doc.extra.clear();
                doc.anchor = 0;
                doc.cursor = doc.text.len_chars();
            }
            (true, "z") => doc.undo(shift),
            (true, "y") => doc.undo(true),
            (true, "c" | "x") => {
                let range = if selection.is_empty() {
                    let row = doc.text.char_to_line(doc.cursor);
                    doc.text.line_to_char(row)
                        ..(doc.text.line_to_char(row) + doc.text.line(row).len_chars())
                } else {
                    selection
                };
                cx.write_to_clipboard(ClipboardItem::new_string(
                    doc.text.slice(range.clone()).to_string(),
                ));
                if key == "x" {
                    doc.replace(range, "");
                }
            }
            (true, "v") => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    let text = if self.single_line {
                        text.replace(['\r', '\n'], " ")
                    } else {
                        text
                    };
                    doc.insert(&text);
                }
            }
            (_, "left" | "right") => {
                let forward = key == "right";
                let position = if !shift && !control && !selection.is_empty() {
                    if forward {
                        selection.end
                    } else {
                        selection.start
                    }
                } else {
                    doc.boundary(forward, control)
                };
                doc.move_to(position, shift);
            }
            (false, "up" | "down" | "pageup" | "pagedown") if !self.single_line => {
                vertical = true;
                let row = doc.text.char_to_line(doc.cursor);
                let column = *self
                    .preferred_column
                    .get_or_insert(doc.cursor - doc.text.line_to_char(row));
                let distance = if key.starts_with("page") {
                    ((f32::from(self.bounds.size.height) / self.appearance.line_height()) as usize)
                        .max(1)
                } else {
                    1
                };
                let row = if key == "up" || key == "pageup" {
                    row.saturating_sub(distance)
                } else {
                    (row + distance).min(doc.text.len_lines() - 1)
                };
                doc.move_to(
                    (doc.text.line_to_char(row) + column).min(doc.line_end(row)),
                    shift,
                );
            }
            (_, "home" | "end") => {
                let row = doc.text.char_to_line(doc.cursor);
                let position = if control {
                    if key == "home" {
                        0
                    } else {
                        doc.text.len_chars()
                    }
                } else if key == "home" {
                    doc.text.line_to_char(row)
                } else {
                    doc.line_end(row)
                };
                doc.move_to(position, shift);
            }
            (_, "backspace" | "delete") => {
                let range = if selection.is_empty() {
                    let end = doc.boundary(key == "delete", control);
                    doc.cursor.min(end)..doc.cursor.max(end)
                } else {
                    selection
                };
                if doc.extra.is_empty() {
                    doc.replace(range, "");
                } else {
                    let saved_cursor = doc.cursor;
                    let mut ranges = doc.extra.clone();
                    for range in &mut ranges {
                        if range.start == range.end {
                            doc.cursor = range.start;
                            let boundary = doc.boundary(key == "delete", control);
                            *range = range.start.min(boundary)..range.end.max(boundary);
                        }
                    }
                    doc.cursor = saved_cursor;
                    doc.extra = ranges;
                    doc.anchor = range.start;
                    doc.cursor = range.end;
                    doc.insert("");
                }
            }
            (false, "enter") if !self.single_line => {
                let row = doc.text.char_to_line(doc.cursor);
                let indentation: String = doc
                    .text
                    .line(row)
                    .chars()
                    .take_while(|ch| *ch == ' ' || *ch == '\t')
                    .collect();
                doc.insert(&format!("\n{indentation}"));
            }
            (false, "tab") if !self.single_line => {
                if shift
                    || doc.text.char_to_line(selection.start)
                        != doc.text.char_to_line(selection.end)
                {
                    doc.indent(shift, self.appearance.tab_size);
                } else {
                    doc.insert(&" ".repeat(self.appearance.tab_size));
                }
            }
            (true, "[" | "]") if !self.single_line => {
                doc.indent(key == "[", self.appearance.tab_size)
            }
            (true, "l") if !self.single_line => {
                if shift {
                    doc.split_selection();
                } else {
                    let row = doc.text.char_to_line(doc.cursor);
                    doc.anchor = doc.text.line_to_char(row);
                    doc.cursor = doc.anchor + doc.text.line(row).len_chars();
                }
            }
            (true, "d") if !self.single_line => {
                doc.duplicate(!shift);
            }
            (true, "k") if shift && !self.single_line => {
                let start = doc
                    .text
                    .line_to_char(doc.text.char_to_line(selection.start));
                let last = doc.text.char_to_line(selection.end);
                let end = doc.text.line_to_char(last) + doc.text.line(last).len_chars();
                doc.replace(start..end, "");
            }
            (false, "escape") if !doc.extra.is_empty() => {
                doc.extra.clear();
            }
            (true, "enter") if !self.single_line => {
                let row = doc.text.char_to_line(doc.cursor);
                let position = if shift {
                    doc.text.line_to_char(row)
                } else {
                    doc.line_end(row)
                };
                doc.replace(position..position, "\n");
                if shift {
                    doc.move_to(position, false);
                }
            }
            _ => return,
        }
        if !vertical {
            self.preferred_column = None;
        }
        self.marked = None;
        if self.has_checkboxes()
            && !shift
            && matches!(
                key,
                "left" | "right" | "up" | "down" | "home" | "end" | "pageup" | "pagedown"
            )
        {
            self.document.select_checkbox_at_cursor();
        }
        self.changed(window, cx);
        cx.stop_propagation();
    }
    fn highlight(&mut self, cx: &mut Context<Self>) {
        if self.single_line {
            return;
        }
        if self.syntax == "txt" || !self.appearance.highlighting {
            if self.color_key.take().is_some() {
                self.generation.fetch_add(1, Ordering::Relaxed);
            }
            self.colors.clear();
            return;
        }
        let rows = ((f32::from(self.bounds.size.height) / self.appearance.line_height()).ceil()
            as usize
            + 2)
        .max(50);
        let end = (self.scroll + rows).min(self.document.text.len_lines());
        let key = (
            self.document.revision,
            self.scroll,
            end,
            self.syntax.clone(),
            self.appearance.theme.clone(),
        );
        if self.color_key.as_ref() == Some(&key) {
            return;
        }
        self.color_key = Some(key.clone());
        let generation = self.generation.clone();
        let ticket = generation.fetch_add(1, Ordering::Relaxed) + 1;
        let text = self.document.text.clone();
        let extension = self.syntax.clone();
        let first = self.scroll;
        let theme = self.appearance.theme();
        let task = cx.background_spawn(async move {
            if extension.eq_ignore_ascii_case("csv") || extension.eq_ignore_ascii_case("tsv") {
                return Some(crate::csv::colors(&text, first, end));
            }
            let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
            let syntax = syntaxes
                .find_syntax_by_extension(&extension)
                .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
            let mut highlighter = HighlightLines::new(syntax, &theme.syntax);
            let mut colors = Vec::new();
            for (row, line) in text.lines().take(end).enumerate() {
                if generation.load(Ordering::Relaxed) != ticket {
                    return None;
                }
                if line.len_bytes() > 32_768 {
                    if row >= first {
                        colors.push(vec![]);
                    }
                    continue;
                }
                let line = line.to_string();
                let spans = highlighter
                    .highlight_line(&line, syntaxes)
                    .unwrap_or_default();
                if row >= first {
                    colors.push(
                        spans
                            .into_iter()
                            .map(|(style, part)| {
                                (
                                    part.len(),
                                    ((style.foreground.r as u32) << 16)
                                        | ((style.foreground.g as u32) << 8)
                                        | style.foreground.b as u32,
                                )
                            })
                            .collect(),
                    );
                }
            }
            Some(colors)
        });
        cx.spawn(async move |this, cx| {
            if let Some(colors) = task.await {
                let _ = this.update(cx, |this, cx| {
                    if this.color_key.as_ref() == Some(&key) {
                        this.colors = colors;
                        this.color_start = first;
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.bounds = bounds;
        self.lines.clear();
        self.checkboxes.clear();
        let rows = (f32::from(bounds.size.height) / self.appearance.line_height()).ceil() as usize;
        let columns =
            (f32::from(bounds.size.width) / self.appearance.cell_width()).ceil() as usize + 8;
        let mut selections = self.document.extra.clone();
        selections.push(self.document.selection());
        let mut caret_targets = Vec::new();
        let spans = if self.wrapping() {
            self.visible_spans(rows, self.wrap_columns())
        } else {
            (self.scroll..(self.scroll + rows).min(self.document.text.len_lines()))
                .map(|row| {
                    let len = self.document.text.line(row).len_chars();
                    let start = self.horizontal.min(len);
                    (row, start..(start + columns).min(len))
                })
                .collect()
        };
        for (visual_row, (row, span)) in spans.into_iter().enumerate() {
            let source = self.document.text.line(row);
            let base = self.document.text.line_to_char(row);
            let start = span.start;
            let end = span.end;
            let tasks =
                if !self.single_line && matches!(self.syntax.as_str(), "txt" | "md" | "markdown") {
                    crate::todo::markers(source, start..end)
                } else {
                    Vec::new()
                };
            let slice = source.slice(start..end);
            let mut display = String::new();
            let mut offsets = Vec::new();
            let mut runs: Vec<TextRun> = Vec::new();
            let mut source_byte = source.char_to_byte(start);
            let spans = row
                .checked_sub(self.color_start)
                .and_then(|i| self.colors.get(i));
            for (index, character) in slice.chars().enumerate() {
                if matches!(character, '\r' | '\n') {
                    break;
                }
                offsets.push((display.len(), base + start + index));
                let mut color = self.appearance.theme().foreground;
                if let Some(spans) = spans {
                    let mut byte = 0;
                    for &(length, rgb) in spans {
                        byte += length;
                        if source_byte < byte {
                            color = rgb;
                            break;
                        }
                    }
                }
                let before = display.len();
                if let Some(task) = tasks
                    .iter()
                    .find(|task| task.range.contains(&(start + index)))
                {
                    display.extend(std::iter::repeat_n(' ', task.display_spaces(start + index)));
                } else if character == '\t' {
                    display.push_str(&" ".repeat(self.appearance.tab_size));
                } else {
                    display.push(character);
                }
                let length = display.len() - before;
                if let Some(last) = runs.last_mut().filter(|run| run.color == rgb(color).into()) {
                    last.len += length;
                } else {
                    runs.push(TextRun {
                        len: length,
                        font: font(self.appearance.font.clone()),
                        color: rgb(color).into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    });
                }
                source_byte += character.len_utf8();
            }
            let actual_end = offsets
                .last()
                .map(|(_, offset)| offset + 1)
                .unwrap_or(base + start);
            offsets.push((display.len(), actual_end));
            let shape = window.text_system().shape_line(
                display.into(),
                px(self.appearance.font_size),
                &runs,
                None,
            );
            let origin = point(
                bounds.left(),
                bounds.top() + px(visual_row as f32 * self.appearance.line_height()),
            );
            let line = VisualLine {
                shape,
                origin,
                offsets,
                start: base + start,
                end: actual_end,
            };
            for selection in &selections {
                if selection.start <= line.end
                    && selection.end > line.start
                    && !selection.is_empty()
                {
                    let left = line.x(selection.start.max(line.start));
                    let right = if selection.end > line.end {
                        bounds.right()
                    } else {
                        line.x(selection.end)
                    };
                    window.paint_quad(fill(
                        Bounds::new(
                            point(left, origin.y),
                            size(right - left, px(self.appearance.line_height())),
                        ),
                        rgb(self.appearance.theme().selection),
                    ));
                }
                if self.focus.is_focused(window)
                    && selection.is_empty()
                    && selection.start >= line.start
                    && (selection.start < line.end
                        || (selection.start == line.end && line.end == self.document.line_end(row)))
                {
                    let p = point(line.x(selection.start), origin.y + px(2.));
                    if !caret_targets.contains(&p) {
                        caret_targets.push(p);
                    }
                }
            }
            if let Err(error) = line.shape.paint(
                origin,
                px(self.appearance.line_height()),
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                eprintln!("Text rendering: {error}");
            }
            if self.focus.is_focused(window)
                && self.document.cursor >= line.start
                && (self.document.cursor < line.end
                    || (self.document.cursor == line.end
                        && line.end == self.document.line_end(row)))
                && !self.document.selection().is_empty()
            {
                caret_targets.push(point(line.x(self.document.cursor), origin.y + px(2.)));
            }
            for task in tasks {
                let left = line.x(base + task.range.start);
                let right = line.x(base + task.range.end);
                let side = px((self.appearance.font_size * 0.85).max(3.)).min(right - left);
                let origin = point(
                    left + (right - left - side) / 2.,
                    origin.y + (px(self.appearance.line_height()) - side) / 2.,
                );
                let checkbox = Bounds::new(origin, size(side, side));
                let color = rgb(if task.checked {
                    self.appearance.theme().accent
                } else {
                    self.appearance.theme().muted
                });
                let stroke = px((self.appearance.font_size / 12.).clamp(1., 2.));
                for rect in [
                    Bounds::new(origin, size(side, stroke)),
                    Bounds::new(origin + point(px(0.), side - stroke), size(side, stroke)),
                    Bounds::new(origin, size(stroke, side)),
                    Bounds::new(origin + point(side - stroke, px(0.)), size(stroke, side)),
                ] {
                    window.paint_quad(fill(rect, color));
                }
                if task.checked {
                    let mut tick = PathBuilder::stroke(stroke);
                    tick.move_to(origin + point(side * 0.2, side * 0.5));
                    tick.line_to(origin + point(side * 0.43, side * 0.73));
                    tick.line_to(origin + point(side * 0.8, side * 0.25));
                    if let Ok(path) = tick.build() {
                        window.paint_path(path, color);
                    }
                }
                self.checkboxes.push((
                    checkbox,
                    base + task.range.start..base + task.range.end,
                    self.document.revision,
                ));
            }
            self.lines.push(line);
        }
        self.carets.paint(
            &caret_targets,
            if self.single_line {
                "off"
            } else {
                &self.appearance.caret_animation
            },
            self.appearance.caret_duration,
            self.appearance.line_height() - 4.,
            self.appearance.theme().foreground,
            window,
        );
        self.highlight(cx);
    }
}
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.appearance = cx.global::<Appearance>().clone();
        self.wrap.checkbox_mode =
            !self.single_line && matches!(self.syntax.as_str(), "txt" | "md" | "markdown");
        let entity = cx.entity();
        div()
            .size_full()
            .overflow_hidden()
            .track_focus(&self.focus)
            .key_context("Editor")
            .cursor(CursorStyle::IBeam)
            .bg(rgb(self.appearance.theme().background))
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if this.single_line || crate::native::right_button_down() {
                        return;
                    }
                    window.focus(&this.focus, cx);
                    let mut previous = this.document.extra.clone();
                    previous.push(this.document.selection());
                    this.rectangle = Some((event.position, previous));
                    this.rectangular_selection(event.position);
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, _, _, _| this.rectangle = None),
            )
            .on_mouse_up_out(
                MouseButton::Middle,
                cx.listener(|this, _, _, _| this.rectangle = None),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    if !event.modifiers.shift
                        && !event.modifiers.control
                        && !event.modifiers.alt
                        && let Some((_, range, revision)) = this
                            .checkboxes
                            .iter()
                            .find(|(bounds, _, _)| bounds.contains(&event.position))
                            .cloned()
                        && revision == this.document.revision
                    {
                        this.selecting = false;
                        this.document.extra.clear();
                        this.document.anchor = range.start;
                        this.document.cursor = range.end;
                        this.marked = None;
                        window.notify_text_input_changed(&this.focus);
                        cx.notify();
                        cx.stop_propagation();
                        return;
                    }
                    this.selecting = true;
                    this.document
                        .move_to(this.hit(event.position), event.modifiers.shift);
                    if event.click_count == 2 {
                        let start = this.document.boundary(false, true);
                        let end = this.document.boundary(true, true);
                        this.document.anchor = start;
                        this.document.cursor = end;
                    } else if event.click_count >= 3 {
                        let row = this.document.text.char_to_line(this.document.cursor);
                        this.document.anchor = this.document.text.line_to_char(row);
                        this.document.cursor =
                            this.document.anchor + this.document.text.line(row).len_chars();
                    }
                    if this.has_checkboxes() && !event.modifiers.shift {
                        this.document.select_checkbox_at_cursor();
                    }
                    this.changed(window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.selecting = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.selecting = false),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if this.rectangle.is_some() && event.pressed_button == Some(MouseButton::Middle) {
                    if let Some((origin, previous)) = this.rectangle.as_mut()
                        && (f32::from(event.position.x - origin.x)).abs()
                            + (f32::from(event.position.y - origin.y)).abs()
                            > 3.
                    {
                        previous.clear();
                    }
                    this.rectangular_selection(event.position);
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if this.selecting && event.pressed_button == Some(MouseButton::Left) {
                    this.document.move_to(this.hit(event.position), true);
                    this.changed(window, cx);
                }
            }))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                if crate::native::right_button_down() || event.modifiers.control {
                    return;
                }
                let delta = event.delta.pixel_delta(px(this.appearance.line_height()));
                if this.wrapping() {
                    this.scroll_wrapped(
                        (-f32::from(delta.y) / this.appearance.line_height()).round() as isize,
                    );
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.modifiers.shift {
                    this.horizontal = (this.horizontal as f32
                        - f32::from(delta.y) / this.appearance.cell_width())
                    .max(0.) as usize;
                } else {
                    this.scroll = ((this.scroll as f32
                        - f32::from(delta.y) / this.appearance.line_height())
                    .max(0.) as usize)
                        .min(this.document.text.len_lines().saturating_sub(1));
                    this.horizontal = (this.horizontal as f32
                        - f32::from(delta.x) / this.appearance.cell_width())
                    .max(0.) as usize;
                }
                cx.notify();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, cx| {
                        let focus = entity.read(cx).focus.clone();
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, entity.clone()),
                            cx,
                        );
                        entity.update(cx, |this, cx| this.paint(bounds, window, cx));
                    },
                )
                .size_full(),
            )
    }
}
impl Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range =
            self.document.utf16_to_char(range.start)..self.document.utf16_to_char(range.end);
        *adjusted = Some(self.document.to_utf16(range.start)..self.document.to_utf16(range.end));
        Some(self.document.text.slice(range).to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self.document.selection();
        Some(UTF16Selection {
            range: self.document.to_utf16(range.start)..self.document.to_utf16(range.end),
            reversed: self.document.cursor < self.document.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|range| self.document.to_utf16(range.start)..self.document.to_utf16(range.end))
    }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        self.changed(window, cx);
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let multiple = range.is_none() && self.marked.is_none() && !self.document.extra.is_empty();
        let range = range
            .map(|r| self.document.utf16_to_char(r.start)..self.document.utf16_to_char(r.end))
            .or(self.marked.take())
            .unwrap_or(self.document.selection());
        let text = if self.single_line {
            text.replace(['\r', '\n'], " ")
        } else {
            text.to_owned()
        };
        if multiple {
            self.document.insert(&text);
        } else {
            self.document.replace(range, &text);
        }
        self.preferred_column = None;
        self.changed(window, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .map(|r| self.document.utf16_to_char(r.start)..self.document.utf16_to_char(r.end))
            .or(self.marked.take())
            .unwrap_or(self.document.selection());
        let start = range.start;
        self.document.replace(range, text);
        self.marked = if text.is_empty() {
            None
        } else {
            Some(start..self.document.cursor)
        };
        if let Some(selection) = selected {
            let utf16 = self.document.to_utf16(start);
            self.document.anchor = self
                .document
                .utf16_to_char(utf16 + selection.start.min(text.encode_utf16().count()));
            self.document.cursor = self
                .document
                .utf16_to_char(utf16 + selection.end.min(text.encode_utf16().count()));
        }
        self.changed(window, cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let position = self.document.utf16_to_char(range.start);
        let line = self
            .lines
            .iter()
            .find(|line| position >= line.start && position <= line.end)?;
        Some(Bounds::new(
            point(line.x(position), line.origin.y),
            size(px(2.), px(self.appearance.line_height())),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.document.to_utf16(self.hit(point)))
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.document.text.len_utf16_cu())
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.document.anchor = self.document.utf16_to_char(range.start);
        self.document.cursor = self.document.utf16_to_char(range.end);
        self.changed(window, cx);
    }
}
