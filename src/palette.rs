use super::*;
use crate::appearance::themes;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Commands,
    Themes,
    Fonts,
    Sizes,
    Languages,
    Caret,
    CaretSpeed,
    Hotkey,
    Indentation,
    Tabs,
    Line,
}
#[derive(Clone)]
struct Entry {
    id: String,
    label: String,
    detail: String,
}
pub(super) struct Palette {
    input: Entity<Editor>,
    mode: Mode,
    selected: usize,
    entries: Vec<Entry>,
    _observe: Subscription,
}
fn normalized(value: &str) -> String {
    value
        .to_lowercase()
        .replace(['é', 'è', 'ê'], "e")
        .replace('à', "a")
        .replace('ô', "o")
}
fn matches(query: &str, candidate: &str) -> bool {
    let candidate = normalized(candidate);
    normalized(query)
        .split_whitespace()
        .all(|word| candidate.contains(word))
}
impl Palette {
    fn filtered(&self, cx: &App) -> Vec<Entry> {
        let query = self.input.read(cx).document.text.to_string();
        if matches!(self.mode, Mode::Sizes | Mode::CaretSpeed)
            && let Ok(value) = query.trim().parse::<f32>()
        {
            let (min, max, unit) = if self.mode == Mode::Sizes {
                (4., 128., "px")
            } else {
                (30., 500., "ms")
            };
            if value.is_finite() && (min..=max).contains(&value) {
                let value = (value * 1000.).round() / 1000.;
                return vec![Entry {
                    id: value.to_string(),
                    label: format!("{value:.3} {unit}"),
                    detail: "Custom value · Enter to apply".into(),
                }];
            }
        }
        self.entries
            .iter()
            .filter(|entry| matches(&query, &format!("{} {}", entry.label, entry.detail)))
            .cloned()
            .collect()
    }
}
impl WinY {
    fn apply_preview(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        if cx.global::<Appearance>() != &appearance {
            cx.set_global(appearance);
            cx.refresh_windows();
        }
    }
    fn palette_entries(&self, mode: Mode, cx: &App) -> Vec<Entry> {
        let pairs: Vec<(String, String, String)> = match mode {
            Mode::Commands => [
                ("toggle-tabs", "View — toggle tab bar", "Alt, V, T"),
                (
                    "hotkey",
                    "Win+Y — global shortcut and Windows startup",
                    "Enable / disable launcher",
                ),
                (
                    "caret",
                    "Caret — animation style",
                    "Off · Linear · Smooth · Spring · Neovide · Trail",
                ),
                ("caret-speed", "Caret — animation duration", "30–500 ms"),
                ("wrap", "View — toggle word wrap", "Alt, V, W"),
                ("unicode", "View — Unicode characters", "Alt, V, U"),
                (
                    "theme",
                    "Theme — choose a theme",
                    "Top 20 Zed theme families + built-in themes · live preview",
                ),
                (
                    "font",
                    "Font — choose a font",
                    "Installed fonts · live preview",
                ),
                (
                    "size",
                    "Font — text size",
                    "4–128 px · precision 0.001 · live preview",
                ),
                ("indentation", "Indentation — tab width", "2 · 4 · 8 spaces"),
                (
                    "syntax",
                    "Language — syntax highlighting",
                    "Choose the tab language",
                ),
                (
                    "highlight",
                    "Syntax highlighting — toggle",
                    "Editor setting",
                ),
                (
                    "black",
                    "Appearance — reset to black and Lilex",
                    "winY defaults",
                ),
                ("new", "New tab", "Ctrl+T"),
                ("open", "Open file", "Ctrl+O"),
                ("save", "Save", "Ctrl+S"),
                ("save-as", "Save as", "Ctrl+Shift+S"),
                ("tabs", "Tabs — switch to tab", "Ctrl+P"),
                ("line", "Go to line", "Ctrl+G"),
                ("find", "Find", "Ctrl+F"),
                ("replace", "Find and replace", "Ctrl+H"),
                ("duplicate", "Duplicate line", "Ctrl+D"),
                ("duplicate-selection", "Duplicate selection", "Ctrl+Shift+D"),
                ("up", "Move line up", "Ctrl+Shift+Up"),
                ("down", "Move line down", "Ctrl+Shift+Down"),
                ("upper", "Convert to UPPERCASE", "Ctrl+K, Ctrl+U"),
                ("lower", "Convert to lowercase", "Ctrl+K, Ctrl+L"),
                (
                    "next-match",
                    "Add next occurrence to selection",
                    "Ctrl+K, Ctrl+D",
                ),
                ("all-matches", "Select all occurrences", "Alt+F3"),
                ("split", "Split selection into lines", "Ctrl+Shift+L"),
                ("join", "Join lines", "Ctrl+J"),
                ("sort", "Sort lines", "F9"),
                ("undo", "Undo", "Ctrl+Z"),
                ("redo", "Redo", "Ctrl+Shift+Z"),
                ("fullscreen", "Toggle fullscreen", "F11"),
                ("menu", "Toggle menu", "Alt"),
                ("close", "Close tab", "Ctrl+W"),
                ("quit", "Quit winY", "Ctrl+Q"),
            ]
            .into_iter()
            .map(|(a, b, c)| (a.into(), b.into(), c.into()))
            .collect(),
            Mode::Themes => themes()
                .iter()
                .map(|theme| {
                    (
                        theme.name.clone(),
                        theme.name.clone(),
                        "Interface theme + syntax colors".into(),
                    )
                })
                .collect(),
            Mode::Fonts => cx
                .text_system()
                .all_font_names()
                .into_iter()
                .map(|name| (name.clone(), name, "Aa Bb 0123456789 — été {}[]".into()))
                .collect(),
            Mode::Sizes => (4..=128)
                .map(|size| (size.to_string(), format!("{size} px"), "Text size".into()))
                .collect(),
            Mode::Indentation => [2, 4, 8]
                .into_iter()
                .map(|size| {
                    (
                        size.to_string(),
                        format!("{size} spaces"),
                        "Tabs and indentation".into(),
                    )
                })
                .collect(),
            Mode::Hotkey => {
                let status = format!(
                    "Win+Y: {} · Windows startup: {}",
                    if crate::hotkey::active() { "on" } else { "off" },
                    if crate::hotkey::startup_enabled() {
                        "on"
                    } else {
                        "off"
                    }
                );
                [
                    ("now", "Enable Win+Y now"),
                    ("startup", "Enable Win+Y and start with Windows"),
                    ("no-startup", "Disable Windows startup only"),
                    ("off", "Disable Win+Y and Windows startup"),
                ]
                .into_iter()
                .map(|(id, label)| (id.into(), label.into(), status.clone()))
                .collect()
            }
            Mode::Caret => ["off", "linear", "smooth", "spring", "neovide", "trail"]
                .into_iter()
                .map(|id| (id.into(), id.into(), "Caret movement animation".into()))
                .collect(),
            Mode::CaretSpeed => [30, 60, 90, 120, 180, 250, 350, 500]
                .into_iter()
                .map(|ms| {
                    (
                        ms.to_string(),
                        format!("{ms} ms"),
                        "Animation duration".into(),
                    )
                })
                .collect(),
            Mode::Languages => [
                ("txt", "Plain text"),
                ("rs", "Rust"),
                ("py", "Python"),
                ("js", "JavaScript"),
                ("ts", "TypeScript"),
                ("json", "JSON"),
                ("html", "HTML"),
                ("css", "CSS"),
                ("md", "Markdown"),
                ("c", "C"),
                ("cpp", "C++"),
                ("sh", "Shell"),
                ("yaml", "YAML"),
                ("toml", "TOML"),
                ("sql", "SQL"),
            ]
            .into_iter()
            .map(|(id, name)| (id.into(), name.into(), id.into()))
            .collect(),
            Mode::Tabs => self
                .tabs
                .iter()
                .enumerate()
                .map(|(i, tab)| {
                    (
                        i.to_string(),
                        tab.title(i + 1, cx),
                        format!("Tab {}", i + 1),
                    )
                })
                .collect(),
            Mode::Line => Vec::new(),
        };
        pairs
            .into_iter()
            .map(|(id, label, detail)| Entry { id, label, detail })
            .collect()
    }
    fn open_palette(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| Editor::new("", true, cx));
        let entries = self.palette_entries(mode, cx);
        let selected = if mode == Mode::Themes {
            entries
                .iter()
                .position(|entry| entry.id == self.appearance.theme)
                .unwrap_or(0)
        } else if mode == Mode::Fonts {
            entries
                .iter()
                .position(|entry| entry.id == self.appearance.font)
                .unwrap_or(0)
        } else {
            0
        };
        let observe = cx.observe(&input, |this, _, cx| {
            if let Some(palette) = this.palette.as_mut() {
                palette.selected = 0;
            }
            this.preview_palette(cx);
            cx.notify();
        });
        self.palette = Some(Palette {
            input: input.clone(),
            mode,
            selected,
            entries,
            _observe: observe,
        });
        window.focus(&input.read(cx).focus.clone(), cx);
        cx.notify();
    }
    fn close_palette(&mut self, commit: bool, window: &mut Window, cx: &mut Context<Self>) {
        if commit {
            self.appearance = cx.global::<Appearance>().clone();
            self.persist(cx);
        } else {
            self.apply_preview(self.appearance.clone(), cx);
        }
        self.palette = None;
        self.focus(window, cx);
        cx.notify();
    }
    fn preview_palette(&mut self, cx: &mut Context<Self>) {
        let Some(palette) = self.palette.as_ref() else {
            return;
        };
        let entries = palette.filtered(cx);
        let Some(entry) = entries.get(palette.selected) else {
            return;
        };
        let mut appearance = cx.global::<Appearance>().clone();
        match palette.mode {
            Mode::Themes => appearance.theme = entry.id.clone(),
            Mode::Fonts => appearance.font = entry.id.clone(),
            Mode::Caret => appearance.caret_animation = entry.id.clone(),
            Mode::CaretSpeed => appearance.caret_duration = entry.id.parse().unwrap_or(120.),
            Mode::Sizes => appearance.font_size = entry.id.parse().unwrap_or(16.),
            _ => return,
        }
        self.apply_preview(appearance, cx);
    }
    fn accept_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(palette) = self.palette.as_ref() else {
            return;
        };
        let mode = palette.mode;
        if mode == Mode::Line {
            if let Ok(row) = palette
                .input
                .read(cx)
                .document
                .text
                .to_string()
                .trim()
                .parse::<usize>()
            {
                self.tabs[self.active].editor.update(cx, |editor, cx| {
                    let row = row
                        .saturating_sub(1)
                        .min(editor.document.text.len_lines() - 1);
                    editor
                        .document
                        .move_to(editor.document.text.line_to_char(row), false);
                    editor.changed(window, cx);
                });
                self.close_palette(false, window, cx);
            }
            return;
        }
        let entries = palette.filtered(cx);
        let Some(entry) = entries.get(palette.selected).cloned() else {
            return;
        };
        match mode {
            Mode::Hotkey => {
                self.close_palette(false, window, cx);
                self.message = "Updating Win+Y settings…".into();
                cx.notify();
                let task = cx.background_spawn(async move { crate::hotkey::configure(&entry.id) });
                cx.spawn(async move |this, cx| {
                    let result = task.await;
                    let _ = this.update(cx, |this, cx| {
                        this.message = result.unwrap_or_else(|e| format!("Win+Y: {e:#}"));
                        cx.notify();
                    });
                })
                .detach();
            }
            Mode::Themes | Mode::Fonts | Mode::Sizes | Mode::Caret | Mode::CaretSpeed => {
                self.preview_palette(cx);
                self.close_palette(true, window, cx);
            }
            Mode::Indentation => {
                let mut appearance = cx.global::<Appearance>().clone();
                appearance.tab_size = entry.id.parse().unwrap_or(4);
                self.apply_preview(appearance, cx);
                self.close_palette(true, window, cx);
            }
            Mode::Languages => {
                self.tabs[self.active].editor.update(cx, |editor, cx| {
                    editor.syntax = entry.id;
                    cx.notify();
                });
                self.close_palette(true, window, cx);
            }
            Mode::Tabs => {
                self.active = entry
                    .id
                    .parse::<usize>()
                    .unwrap_or(0)
                    .min(self.tabs.len() - 1);
                self.close_palette(true, window, cx);
            }
            Mode::Commands => {
                let next_mode = match entry.id.as_str() {
                    "theme" => Some(Mode::Themes),
                    "font" => Some(Mode::Fonts),
                    "size" => Some(Mode::Sizes),
                    "syntax" => Some(Mode::Languages),
                    "caret" => Some(Mode::Caret),
                    "hotkey" => Some(Mode::Hotkey),
                    "caret-speed" => Some(Mode::CaretSpeed),
                    "indentation" => Some(Mode::Indentation),
                    "tabs" => Some(Mode::Tabs),
                    "line" => Some(Mode::Line),
                    _ => None,
                };
                if let Some(mode) = next_mode {
                    self.open_palette(mode, window, cx);
                    return;
                }
                self.close_palette(false, window, cx);
                match entry.id.as_str() {
                    "wrap" => self.toggle_wrap(window, cx),
                    "toggle-tabs" => self.toggle_tabs(window, cx),
                    "unicode" => self.toggle_unicode(window, cx),
                    "highlight" => {
                        self.appearance.highlighting = !self.appearance.highlighting;
                        self.apply_preview(self.appearance.clone(), cx);
                    }
                    "black" => {
                        self.appearance = Appearance::default();
                        self.apply_preview(self.appearance.clone(), cx);
                    }
                    "new" => {
                        self.new_tab(cx);
                        self.focus(window, cx);
                    }
                    "open" => self.open(window, cx),
                    "save" => self.save(false, cx),
                    "save-as" => self.save(true, cx),
                    "find" | "replace" => {
                        self.search = true;
                        self.replace = entry.id == "replace";
                        window.focus(&self.query.read(cx).focus.clone(), cx);
                    }
                    "fullscreen" => window.toggle_fullscreen(),
                    "menu" => self.menu = !self.menu,
                    "close" => self.close_tab(window, cx),
                    "quit" => {
                        self.persist(cx);
                        cx.quit();
                    }
                    command => {
                        self.tabs[self.active].editor.update(cx, |editor, cx| {
                            match command {
                                "duplicate" => editor.document.duplicate(true),
                                "duplicate-selection" => editor.document.duplicate(false),
                                "up" => editor.document.move_lines(false),
                                "down" => editor.document.move_lines(true),
                                "upper" => editor.document.transform_case(true),
                                "lower" => editor.document.transform_case(false),
                                "next-match" => editor.document.select_next_occurrence(),
                                "all-matches" => editor.document.select_all_occurrences(),
                                "split" => editor.document.split_selection(),
                                "join" => editor.document.join_lines(),
                                "sort" => editor.document.sort_lines(true),
                                "undo" => editor.document.undo(false),
                                "redo" => editor.document.undo(true),
                                _ => {}
                            }
                            editor.changed(window, cx);
                        });
                    }
                }
                self.persist(cx);
                cx.notify();
            }
            Mode::Line => {}
        }
    }
    pub(super) fn palette_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.prefer_character_input {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if modifiers.control && matches!(key, "p" | "g") {
            self.zoom_target = None;
            let mode = if key == "g" {
                Mode::Line
            } else if modifiers.shift {
                Mode::Commands
            } else {
                Mode::Tabs
            };
            if self.palette.is_some() {
                self.close_palette(false, window, cx);
            }
            self.open_palette(mode, window, cx);
            cx.stop_propagation();
            return;
        }
        if self.palette.is_none() {
            if modifiers.control && matches!(key, "+" | "=" | "-" | "0") {
                let mut appearance = self.appearance.clone();
                appearance.font_size = match key {
                    "-" => (appearance.font_size - 1.).max(4.),
                    "0" => 16.,
                    _ => (appearance.font_size + 1.).min(128.),
                };
                self.zoom_target = None;
                self.appearance = appearance.clone();
                self.apply_preview(appearance, cx);
                self.persist(cx);
                cx.notify();
                cx.stop_propagation();
            }
            return;
        }
        match key {
            "escape" => self.close_palette(false, window, cx),
            "enter" => self.accept_palette(window, cx),
            "up" | "down" | "pageup" | "pagedown" => {
                let palette = self.palette.as_mut().expect("open palette");
                let count = palette.filtered(cx).len();
                if count > 0 {
                    let step = if key.starts_with("page") { 10 } else { 1 };
                    palette.selected = if key == "up" || key == "pageup" {
                        palette.selected.saturating_sub(step)
                    } else {
                        (palette.selected + step).min(count - 1)
                    };
                }
                self.preview_palette(cx);
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    pub(super) fn render_palette(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let palette = self.palette.as_ref()?;
        let mode = palette.mode;
        let appearance = cx.global::<Appearance>().clone();
        let theme = appearance.theme();
        let entries = palette.filtered(cx);
        let count = entries.len();
        let selected = palette.selected.min(count.saturating_sub(1));
        let title = match mode {
            Mode::Commands => "Commands and settings",
            Mode::Themes => "Themes — live preview",
            Mode::Fonts => "Fonts — live preview",
            Mode::Sizes => "Text size — live preview",
            Mode::Hotkey => "Win+Y — global shortcut and Windows startup",
            Mode::Caret => "Caret animation",
            Mode::CaretSpeed => "Caret animation duration",
            Mode::Languages => "Language",
            Mode::Indentation => "Indentation",
            Mode::Tabs => "Tabs",
            Mode::Line => "Go to line",
        };
        let width = f32::from(window.bounds().size.width).min(720.) - 32.;
        let height = (f32::from(window.bounds().size.height) - 170.).max(60.);
        let visible = ((height / 48.) as usize).clamp(1, 10);
        let first = selected.saturating_sub(visible / 2);
        Some(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0x00000055))
                .flex()
                .justify_center()
                .items_start()
                .pt(px(56.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| this.close_palette(false, window, cx)),
                )
                .child(
                    div()
                        .w(px(width))
                        .h_auto()
                        .flex()
                        .flex_col()
                        .bg(rgb(theme.surface))
                        .border_1()
                        .border_color(rgb(theme.border))
                        .rounded_lg()
                        .shadow_lg()
                        .overflow_hidden()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .px_4()
                                .pt_3()
                                .pb_2()
                                .text_color(rgb(theme.muted))
                                .child(title),
                        )
                        .child(
                            div()
                                .mx_3()
                                .mb_2()
                                .p_2()
                                .h(px(appearance.line_height() + 16.))
                                .bg(rgb(theme.background))
                                .border_1()
                                .border_color(rgb(theme.accent))
                                .rounded_md()
                                .child(palette.input.clone()),
                        )
                        .children(
                            entries
                                .into_iter()
                                .enumerate()
                                .skip(first)
                                .take(visible)
                                .map(|(index, entry)| {
                                    let preview = if mode == Mode::Themes {
                                        themes().iter().find(|theme| theme.name == entry.id)
                                    } else {
                                        None
                                    };
                                    let row_background = preview
                                        .map(|theme| theme.background)
                                        .unwrap_or(if index == selected {
                                            theme.selection
                                        } else {
                                            theme.surface
                                        });
                                    let row_foreground = preview
                                        .map(|theme| theme.foreground)
                                        .unwrap_or(theme.foreground);
                                    let swatches = preview
                                        .map(|theme| theme.swatches.clone())
                                        .unwrap_or_default();
                                    div()
                                        .id(("palette-row", index))
                                        .mx_2()
                                        .my(px(1.))
                                        .px_3()
                                        .h(px(46.))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .rounded_md()
                                        .bg(rgb(row_background))
                                        .border_1()
                                        .border_color(rgb(if index == selected {
                                            theme.accent
                                        } else {
                                            row_background
                                        }))
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap_1()
                                                .text_color(rgb(row_foreground))
                                                .child(entry.label)
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .when(mode == Mode::Fonts, |div| {
                                                            div.font_family(entry.id.clone())
                                                        })
                                                        .child(entry.detail),
                                                ),
                                        )
                                        .child(div().flex().gap_1().children(
                                            swatches.into_iter().map(|color| {
                                                div()
                                                    .w(px(12.))
                                                    .h(px(12.))
                                                    .rounded_sm()
                                                    .bg(rgb(color))
                                            }),
                                        ))
                                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                            if let Some(palette) = this.palette.as_mut()
                                                && palette.selected != index
                                            {
                                                palette.selected = index;
                                                this.preview_palette(cx);
                                                cx.notify();
                                            }
                                        }))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, window, cx| {
                                                if let Some(palette) = this.palette.as_mut() {
                                                    palette.selected = index;
                                                }
                                                this.accept_palette(window, cx);
                                                cx.stop_propagation();
                                            }),
                                        )
                                }),
                        )
                        .when(count == 0, |container| {
                            container.child(div().px_4().py_3().child(if mode == Mode::Line {
                                "Type a line number, then Enter"
                            } else {
                                "No results"
                            }))
                        })
                        .child(
                            div()
                                .px_4()
                                .py_2()
                                .text_size(px(11.))
                                .text_color(rgb(theme.muted))
                                .child(format!(
                                    "↑↓ Preview    Enter Apply    Esc Cancel       {count} choices"
                                )),
                        ),
                )
                .into_any_element(),
        )
    }
}
