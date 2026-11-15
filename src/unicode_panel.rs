use super::*;
use std::sync::OnceLock;

struct Character {
    glyph: String,
    name: &'static str,
    code: &'static str,
    alt: &'static str,
}
fn characters() -> &'static [Character] {
    static DATA: OnceLock<Vec<Character>> = OnceLock::new();
    DATA.get_or_init(|| {
        include_str!("../assets/unicode.tsv")
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let code = fields.next()?;
                let glyph = char::from_u32(u32::from_str_radix(code, 16).ok()?)?.to_string();
                Some(Character {
                    glyph,
                    code,
                    name: fields.next()?,
                    alt: fields.next().unwrap_or(""),
                })
            })
            .collect()
    })
}
pub(super) struct UnicodePanel {
    input: Entity<Editor>,
    matches: Vec<usize>,
    hover: Option<usize>,
    _observe: Subscription,
}
impl WinY {
    pub(super) fn toggle_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance.show_tabs = !self.appearance.show_tabs;
        let mut visual = cx.global::<Appearance>().clone();
        visual.show_tabs = self.appearance.show_tabs;
        cx.set_global(visual);
        self.menu = false;
        self.view_menu = false;
        self.focus(window, cx);
        self.persist(cx);
        cx.refresh_windows();
    }

    pub(super) fn toggle_wrap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance.word_wrap = !self.appearance.word_wrap;
        cx.set_global(self.appearance.clone());
        self.menu = false;
        self.view_menu = false;
        self.tabs[self.active].editor.update(cx, |editor, cx| {
            editor.appearance = cx.global::<Appearance>().clone();
            editor.reveal();
            cx.notify();
        });
        self.focus(window, cx);
        self.persist(cx);
        cx.refresh_windows();
    }
    pub(super) fn toggle_unicode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = false;
        self.view_menu = false;
        if self.unicode.take().is_some() {
            self.focus(window, cx);
        } else {
            let input = cx.new(|cx| Editor::new("", true, cx));
            let observe = cx.observe(&input, |this, input, cx| {
                let query = input.read(cx).document.text.to_string().to_uppercase();
                if let Some(panel) = this.unicode.as_mut() {
                    panel.matches = characters()
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| {
                            query.split_whitespace().all(|q| {
                                c.name.contains(q)
                                    || c.glyph == q
                                    || c.code == q.trim_start_matches("U+")
                            })
                        })
                        .map(|(i, _)| i)
                        .collect();
                    panel.hover = None;
                    cx.notify();
                }
            });
            window.focus(&input.read(cx).focus.clone(), cx);
            self.unicode = Some(UnicodePanel {
                input,
                matches: (0..characters().len()).collect(),
                hover: None,
                _observe: observe,
            });
        }
        cx.notify();
    }
    pub(super) fn capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let m = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if m.alt {
            self.alt_used = true;
        }
        if !event.prefer_character_input && !m.control && self.palette.is_none() {
            let handled = match key {
                "v" if self.menu || m.alt => {
                    self.menu = true;
                    self.view_menu = true;
                    true
                }
                "w" if self.menu && self.view_menu => {
                    self.toggle_wrap(window, cx);
                    true
                }
                "t" if self.menu && self.view_menu => {
                    self.toggle_tabs(window, cx);
                    true
                }
                "u" if self.menu && self.view_menu => {
                    self.toggle_unicode(window, cx);
                    true
                }
                "escape" if self.menu => {
                    self.menu = false;
                    self.view_menu = false;
                    true
                }
                "escape" if self.unicode.is_some() => {
                    self.toggle_unicode(window, cx);
                    true
                }
                _ => false,
            };
            if handled {
                window.prevent_default();
                cx.stop_propagation();
                cx.notify();
                return;
            }
        }
        self.palette_key(event, window, cx);
    }
    pub(super) fn render_unicode(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.unicode.as_ref()?;
        let theme = cx.global::<Appearance>().theme();
        let count = panel.matches.len();
        let info = panel
            .hover
            .map(|i| {
                let c = &characters()[i];
                if c.alt.is_empty() {
                    format!("{} · U+{}\n{}", c.glyph, c.code, c.name)
                } else {
                    format!(
                        "{} · Alt+{} · U+{}\n{}{}",
                        c.glyph,
                        c.alt,
                        c.code,
                        c.name,
                        if c.alt.starts_with('0') {
                            " · Windows-1252"
                        } else {
                            ""
                        }
                    )
                }
            })
            .unwrap_or_else(|| format!("{count} characters\nSearch names, symbols or U+ codes"));
        Some(
            div()
                .w(px(252.))
                .h_full()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .border_l_1()
                .border_color(rgb(theme.border))
                .bg(rgb(theme.surface))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .px_2()
                        .py_1()
                        .child("Unicode")
                        .child(
                            div()
                                .id("unicode-close")
                                .cursor_pointer()
                                .child("×")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_unicode(window, cx)
                                })),
                        ),
                )
                .child(
                    div()
                        .px_2()
                        .text_size(px(10.))
                        .text_color(rgb(theme.muted))
                        .child("Search · e.g. arrow"),
                )
                .child(div().mx_2().my_1().h(px(26.)).child(panel.input.clone()))
                .child(
                    uniform_list(
                        "unicode-grid",
                        count.div_ceil(7),
                        cx.processor(|this, rows: std::ops::Range<usize>, _, cx| {
                            let panel = this.unicode.as_ref().expect("Unicode panel");
                            let theme = cx.global::<Appearance>().theme();
                            rows.map(|row| {
                                div()
                                    .h(px(32.))
                                    .flex()
                                    .px_1()
                                    .children(
                                        panel.matches.iter().skip(row * 7).take(7).copied().map(
                                            |index| {
                                                let c = &characters()[index];
                                                div()
                                                    .id(("unicode", index))
                                                    .w(px(34.))
                                                    .h(px(32.))
                                                    .flex()
                                                    .justify_center()
                                                    .items_center()
                                                    .rounded_sm()
                                                    .cursor_pointer()
                                                    .font_family("Segoe UI Symbol")
                                                    .text_size(px(20.))
                                                    .hover(|s| s.bg(rgb(theme.selection)))
                                                    .child(c.glyph.clone())
                                                    .on_hover(cx.listener(
                                                        move |this, hovered, _, cx| {
                                                            if *hovered {
                                                                if let Some(p) =
                                                                    this.unicode.as_mut()
                                                                {
                                                                    p.hover = Some(index);
                                                                }
                                                                cx.notify();
                                                            }
                                                        },
                                                    ))
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.tabs[this.active].editor.update(
                                                                cx,
                                                                |editor, cx| {
                                                                    editor.document.insert(
                                                                        &characters()[index].glyph,
                                                                    );
                                                                    editor.changed(window, cx);
                                                                },
                                                            );
                                                            this.focus(window, cx);
                                                        },
                                                    ))
                                            },
                                        ),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                        }),
                    )
                    .flex_1()
                    .min_h_0(),
                )
                .child(
                    div()
                        .px_2()
                        .py_2()
                        .h(px(70.))
                        .overflow_hidden()
                        .text_size(px(10.))
                        .text_color(rgb(theme.muted))
                        .child(info),
                )
                .into_any_element(),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::characters;
    #[test]
    fn unicode_names_and_alt_codes() {
        let data = characters();
        assert!(data.iter().filter(|c| c.name.contains("ARROW")).count() > 200);
        assert_eq!(data.iter().find(|c| c.glyph == "—").unwrap().alt, "0151");
        assert_eq!(data.iter().find(|c| c.glyph == "♥").unwrap().alt, "3");
        assert!(data.iter().find(|c| c.glyph == "⟹").unwrap().alt.is_empty());
    }
}
