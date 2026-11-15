use crate::{
    editor::Editor,
    session::{self, Manifest, Snapshot, TabMeta, Writer},
};
use gpui::{prelude::*, *};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[path = "palette.rs"]
mod palette;
#[path = "unicode_panel.rs"]
mod unicode_panel;
use crate::appearance::Appearance;
use palette::Palette;
use unicode_panel::UnicodePanel;

struct Tab {
    meta: TabMeta,
    editor: Entity<Editor>,
    _observe: Subscription,
}
impl Tab {
    fn title(&self, number: usize, cx: &App) -> String {
        self.meta
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.editor.read(cx).document.tab_title(number))
    }
}
pub struct WinY {
    tabs: Vec<Tab>,
    active: usize,
    closed: Vec<Tab>,
    writer: Writer,
    menu: bool,
    view_menu: bool,
    unicode: Option<UnicodePanel>,
    alt_pressed: bool,
    alt_used: bool,
    search: bool,
    replace: bool,
    query: Entity<Editor>,
    replacement: Entity<Editor>,
    message: String,
    next_id: u64,
    search_generation: u64,
    appearance: Appearance,
    palette: Option<Palette>,
    drag: Option<crate::native::WindowDrag>,
    topmost: bool,
    toast: Option<(String, std::time::Instant)>,
    zoom_target: Option<f32>,
    zoom_tick: std::time::Instant,
    width_motion: Option<crate::window_motion::WidthMotion>,
    width_tick: std::time::Instant,
}
impl WinY {
    pub fn new(manifest: Manifest, texts: Vec<String>, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            tabs: vec![],
            active: manifest.active,
            closed: vec![],
            writer: Writer::new(session::directory()),
            menu: false,
            view_menu: false,
            unicode: None,
            alt_pressed: false,
            alt_used: false,
            search: false,
            replace: false,
            query: cx.new(|cx| Editor::new("", true, cx)),
            replacement: cx.new(|cx| Editor::new("", true, cx)),
            message: String::new(),
            search_generation: 0,
            appearance: cx.global::<Appearance>().clone(),
            palette: None,
            drag: None,
            topmost: false,
            toast: None,
            zoom_target: None,
            zoom_tick: std::time::Instant::now(),
            width_motion: None,
            width_tick: std::time::Instant::now(),
            next_id: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        };
        for (meta, text) in manifest.tabs.into_iter().zip(texts) {
            this.add(meta, &text, cx);
        }
        if this.tabs.is_empty() {
            this.new_tab(cx);
        }
        this.active = this.active.min(this.tabs.len() - 1);
        this.persist(cx);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        while let Ok(error) = this.writer.errors.try_recv() {
                            this.message = error;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        this
    }
    fn add(&mut self, meta: TabMeta, text: &str, cx: &mut Context<Self>) {
        let editor = cx.new(|cx| {
            let mut editor = Editor::new(text, false, cx);
            editor.document.cursor = meta.cursor.min(editor.document.text.len_chars());
            editor.document.anchor = meta.anchor.min(editor.document.text.len_chars());
            editor.scroll = meta.scroll.min(editor.document.text.len_lines() - 1);
            editor.syntax = meta.syntax.clone();
            editor
        });
        let previous = std::cell::RefCell::new(None);
        let observe = cx.observe(&editor, move |this, editor, cx| {
            let editor = editor.read(cx);
            let state = (
                editor.document.revision,
                editor.document.cursor,
                editor.document.anchor,
                editor.scroll,
                editor.syntax.clone(),
            );
            if previous.borrow().as_ref() != Some(&state) {
                *previous.borrow_mut() = Some(state);
                this.persist(cx);
                cx.notify();
            }
        });
        self.tabs.push(Tab {
            meta,
            editor,
            _observe: observe,
        });
    }
    fn new_tab(&mut self, cx: &mut Context<Self>) {
        self.next_id += 1;
        self.add(
            TabMeta {
                id: self.next_id,
                title: (self.tabs.len() + 1).to_string(),
                path: None,
                cursor: 0,
                anchor: 0,
                scroll: 0,
                syntax: "txt".into(),
            },
            "",
            cx,
        );
        self.active = self.tabs.len() - 1;
    }
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.tabs[self.active].editor.read(cx).focus.clone(), cx);
    }
    fn persist(&mut self, cx: &App) {
        let mut manifest = Manifest {
            active: self.active,
            tabs: vec![],
            appearance: self.appearance.clone(),
        };
        let mut texts = Vec::new();
        for (index, tab) in self.tabs.iter().enumerate() {
            let editor = tab.editor.read(cx);
            let mut meta = tab.meta.clone();
            meta.cursor = editor.document.cursor;
            meta.anchor = editor.document.anchor;
            meta.scroll = editor.scroll;
            meta.syntax = editor.syntax.clone();
            meta.title = tab.title(index + 1, cx);
            manifest.tabs.push(meta);
            texts.push((
                tab.meta.id,
                editor.document.revision,
                editor.document.text.clone(),
            ));
        }
        for tab in &self.closed {
            let editor = tab.editor.read(cx);
            texts.push((
                tab.meta.id,
                editor.document.revision,
                editor.document.text.clone(),
            ));
        }
        self.writer.submit(Snapshot { manifest, texts });
        while let Ok(error) = self.writer.errors.try_recv() {
            self.message = error;
        }
    }
    fn switch(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.active =
            (self.active + if forward { 1 } else { self.tabs.len() - 1 }) % self.tabs.len();
        self.focus(window, cx);
        self.persist(cx);
        cx.notify();
    }
    fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.closed.push(self.tabs.remove(self.active));
        if self.closed.len() > 20 {
            self.closed.remove(0);
        }
        if self.tabs.is_empty() {
            self.new_tab(cx);
        }
        self.active = self.active.min(self.tabs.len() - 1);
        self.focus(window, cx);
        self.persist(cx);
        cx.notify();
    }
    fn find(&mut self, backwards: bool, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query.read(cx).document.text.to_string();
        if let Some(target) = self.zoom_target {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.zoom_tick).as_secs_f32().min(0.05);
            self.zoom_tick = now;
            let mut appearance = cx.global::<Appearance>().clone();
            let remaining = target - appearance.font_size;
            appearance.font_size = if remaining.abs() < 0.001 {
                target
            } else {
                appearance.font_size + remaining * (1. - (-dt * 32.).exp())
            };
            cx.set_global(appearance);
            if remaining.abs() < 0.001 {
                self.zoom_target = None;
            } else {
                window.request_animation_frame();
            }
        }
        let toast = self.toast.as_ref().and_then(|(text, start)| {
            let elapsed = start.elapsed().as_secs_f32();
            (elapsed < 1.).then(|| (text.clone(), ((1. - elapsed) / 0.35).clamp(0., 1.)))
        });
        if toast.is_some() {
            window.request_animation_frame();
        }
        let editor = self.tabs[self.active].editor.clone();
        let current = editor.read(cx);
        let revision = current.document.revision;
        let from = if backwards {
            current.document.selection().start
        } else {
            current.document.selection().end
        };
        let mut document = crate::document::Document::new("");
        document.text = current.document.text.clone();
        self.search_generation += 1;
        let generation = self.search_generation;
        let task = cx.background_spawn(async move { document.find(&query, from, backwards) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if generation != this.search_generation
                    || editor.read(cx).document.revision != revision
                {
                    return;
                }
                if let Some(range) = result {
                    editor.update(cx, |editor, cx| {
                        editor.document.extra.clear();
                        editor.document.anchor = range.start;
                        editor.document.cursor = range.end;
                        editor.changed(window, cx);
                    });
                    this.message.clear();
                } else {
                    this.message = "No matches".into();
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn replace_matches(&mut self, all: bool, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query.read(cx).document.text.to_string();
        let replacement = self.replacement.read(cx).document.text.to_string();
        if all {
            if let Some(target) = self.zoom_target {
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.zoom_tick).as_secs_f32().min(0.05);
                self.zoom_tick = now;
                let mut appearance = cx.global::<Appearance>().clone();
                let remaining = target - appearance.font_size;
                appearance.font_size = if remaining.abs() < 0.001 {
                    target
                } else {
                    appearance.font_size + remaining * (1. - (-dt * 32.).exp())
                };
                cx.set_global(appearance);
                if remaining.abs() < 0.001 {
                    self.zoom_target = None;
                } else {
                    window.request_animation_frame();
                }
            }
            let toast = self.toast.as_ref().and_then(|(text, start)| {
                let elapsed = start.elapsed().as_secs_f32();
                (elapsed < 1.).then(|| (text.clone(), ((1. - elapsed) / 0.35).clamp(0., 1.)))
            });
            if toast.is_some() {
                window.request_animation_frame();
            }
            let editor = self.tabs[self.active].editor.clone();
            let revision = editor.read(cx).document.revision;
            let mut document = crate::document::Document::new("");
            document.text = editor.read(cx).document.text.clone();
            let task = cx.background_spawn(async move {
                let count = document.replace_all(&query, &replacement);
                (count, document.text)
            });
            cx.spawn_in(window, async move |this, cx| {
                let (count, text) = task.await;
                let _ = this.update_in(cx, |this, window, cx| {
                    if editor.read(cx).document.revision != revision {
                        this.message = "Text changed: run replace again.".into();
                        cx.notify();
                        return;
                    }
                    if count > 0 {
                        editor.update(cx, |editor, cx| {
                            editor.document.replace_rope(text);
                            editor.changed(window, cx);
                        });
                    }
                    this.message = format!("{count} replacement(s)");
                    cx.notify();
                });
            })
            .detach();
            return;
        }
        let count = self.tabs[self.active].editor.update(cx, |editor, cx| {
            let count = if all {
                editor.document.replace_all(&query, &replacement)
            } else if !query.is_empty()
                && editor.document.text.slice(editor.document.selection()) == query
            {
                editor
                    .document
                    .replace(editor.document.selection(), &replacement);
                1
            } else {
                0
            };
            editor.changed(window, cx);
            count
        });
        if !all {
            self.find(false, window, cx);
        }
        self.message = format!("{count} replacement(s)");
        cx.notify();
    }
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = window.window_handle();
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                let _ = cx.update_window(handle, |_, window, cx| {
                    this.update(cx, |this, cx| this.open_paths(paths, window, cx))
                });
            }
            Ok(Err(e)) => {
                let _ = this.update(cx, |this, cx| {
                    this.message = e.to_string();
                    cx.notify();
                });
            }
            _ => {}
        })
        .detach();
    }
    fn open_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let handle = window.window_handle();
        let task = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(|path| {
                    let text = std::fs::read_to_string(&path);
                    (path, text)
                })
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this, cx| {
            let files = task.await;
            let _ = cx.update_window(handle, |_, window, cx| {
                this.update(cx, |this, cx| {
                    for (path, text) in files {
                        match text {
                            Ok(text) => {
                                this.next_id += 1;
                                let syntax = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("txt")
                                    .to_lowercase();
                                this.add(
                                    TabMeta {
                                        id: this.next_id,
                                        title: String::new(),
                                        path: Some(path),
                                        cursor: 0,
                                        anchor: 0,
                                        scroll: 0,
                                        syntax,
                                    },
                                    &text,
                                    cx,
                                );
                                this.active = this.tabs.len() - 1;
                            }
                            Err(e) => {
                                this.message = format!("Could not open {}: {e}", path.display())
                            }
                        }
                    }
                    this.persist(cx);
                    this.focus(window, cx);
                    cx.notify();
                })
            });
        })
        .detach();
    }
    fn save(&mut self, save_as: bool, cx: &mut Context<Self>) {
        let tab = &self.tabs[self.active];
        let id = tab.meta.id;
        let path = if save_as { None } else { tab.meta.path.clone() };
        let text = tab.editor.read(cx).document.text.clone();
        let receiver = if path.is_none() {
            Some(
                cx.prompt_for_new_path(
                    &std::env::var_os("USERPROFILE")
                        .map(PathBuf::from)
                        .unwrap_or_else(session::directory),
                    Some("note.txt"),
                ),
            )
        } else {
            None
        };
        cx.spawn(async move |this, cx| {
            let result = async {
                let path = if let Some(receiver) = receiver {
                    receiver.await??
                } else {
                    path
                };
                if let Some(path) = path {
                    let target = path.clone();
                    cx.background_spawn(async move {
                        session::atomic_write(&target, |file| {
                            use std::io::Write;
                            for chunk in text.chunks() {
                                file.write_all(chunk.as_bytes())?;
                            }
                            Ok(())
                        })
                    })
                    .await?;
                    this.update(cx, |this, cx| {
                        if let Some(tab) = this.tabs.iter_mut().find(|tab| tab.meta.id == id) {
                            tab.meta.title = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            let syntax = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("txt")
                                .to_owned();
                            tab.meta.path = Some(path);
                            tab.editor.update(cx, |editor, cx| {
                                editor.syntax = syntax;
                                cx.notify();
                            });
                        }
                        this.message = "Saved".into();
                        this.persist(cx);
                        cx.notify();
                    })?;
                }
                anyhow::Ok(())
            }
            .await;
            if let Err(error) = result {
                let _ = this.update(cx, |this, cx| {
                    this.message = format!("Could not save: {error:#}");
                    cx.notify();
                });
            }
        })
        .detach();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if self.palette.is_some() {
            return;
        }
        if modifiers.alt {
            self.alt_used = true;
        }

        match (modifiers.control, modifiers.shift, key) {
            (true, false, "t" | "n") => {
                self.new_tab(cx);
                self.focus(window, cx);
            }
            (true, true, "t") => {
                if let Some(tab) = self.closed.pop() {
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                    self.focus(window, cx);
                }
            }
            (true, _, "w") => self.close_tab(window, cx),
            (true, shift, "tab") => self.switch(!shift, window, cx),
            (true, _, "pagedown") => self.switch(true, window, cx),
            (true, _, "pageup") => self.switch(false, window, cx),
            (true, _, "o") => self.open(window, cx),
            (true, shift, "s") => self.save(shift, cx),
            (true, _, "f" | "h") => {
                self.search = true;
                self.replace = key == "h";
                let selection = self.tabs[self.active].editor.read(cx).document.selection();
                if !selection.is_empty() {
                    let text = self.tabs[self.active]
                        .editor
                        .read(cx)
                        .document
                        .text
                        .slice(selection)
                        .to_string();
                    self.query.update(cx, |editor, cx| {
                        editor
                            .document
                            .replace(0..editor.document.text.len_chars(), &text);
                        editor.document.anchor = 0;
                        cx.notify();
                    });
                }
                window.focus(&self.query.read(cx).focus.clone(), cx);
            }
            (false, shift, "f3") => self.find(shift, window, cx),
            (control, shift, "enter")
                if self.search
                    && (self.query.read(cx).focus.is_focused(window)
                        || self.replacement.read(cx).focus.is_focused(window)) =>
            {
                if control {
                    self.replace_matches(shift, window, cx);
                } else {
                    self.find(shift, window, cx);
                }
            }
            (false, shift, "tab") if self.search => {
                if self.replace && !shift && self.query.read(cx).focus.is_focused(window) {
                    window.focus(&self.replacement.read(cx).focus.clone(), cx);
                } else {
                    window.focus(&self.query.read(cx).focus.clone(), cx);
                }
            }
            (false, _, "escape") => {
                self.search = false;
                self.menu = false;
                self.message.clear();
                self.focus(window, cx);
            }
            (false, _, "f11") => window.toggle_fullscreen(),
            (true, _, "q") => {
                self.persist(cx);
                cx.quit();
            }
            _ => return,
        }
        self.persist(cx);
        cx.notify();
        cx.stop_propagation();
    }
}
impl Render for WinY {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(motion) = self.width_motion.as_mut() {
            let now = std::time::Instant::now();
            let settled = motion.advance(now.duration_since(self.width_tick).as_secs_f32());
            self.width_tick = now;
            let mut dimensions = window.bounds().size;
            let scale = window.scale_factor();
            if (f32::from(dimensions.width) * scale).round() != (motion.current * scale).round() {
                dimensions.width = px(motion.current);
                window.resize(dimensions);
            }
            if settled {
                self.width_motion = None;
            } else {
                window.request_animation_frame();
            }
        }
        if let Some(target) = self.zoom_target {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.zoom_tick).as_secs_f32().min(0.05);
            self.zoom_tick = now;
            let mut appearance = cx.global::<Appearance>().clone();
            let remaining = target - appearance.font_size;
            appearance.font_size = if remaining.abs() < 0.001 {
                target
            } else {
                appearance.font_size + remaining * (1. - (-dt * 32.).exp())
            };
            cx.set_global(appearance);
            if remaining.abs() < 0.001 {
                self.zoom_target = None;
            } else {
                window.request_animation_frame();
            }
        }
        let toast = self.toast.as_ref().and_then(|(text, start)| {
            let elapsed = start.elapsed().as_secs_f32();
            (elapsed < 1.).then(|| (text.clone(), ((1. - elapsed) / 0.35).clamp(0., 1.)))
        });
        if toast.is_some() {
            window.request_animation_frame();
        }
        let editor = self.tabs[self.active].editor.clone();
        let appearance = cx.global::<Appearance>().clone();
        let theme = appearance.theme();
        let palette = self.render_palette(window, cx);
        let unicode = self.render_unicode(cx);
        div()
            .id("winy-root")
            .relative()
            .size_full()
            .overflow_hidden()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                this.open_paths(paths.paths().to_vec(), window, cx)
            }))
            .flex()
            .flex_col()
            .bg(rgb(theme.background))
            .opacity(appearance.opacity)
            .text_color(rgb(theme.foreground))
            .font_family(appearance.font)
            .text_size(px(13.))
            .on_key_down(cx.listener(Self::key))
            .capture_key_down(cx.listener(Self::capture_key))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(|this, _, window, cx| {
                    if crate::native::right_button_down() {
                        this.drag = None;
                        let enabled = !this.topmost;
                        match crate::native::set_topmost(window, enabled) {
                            Ok(()) => {
                                this.topmost = enabled;
                                this.toast = Some((
                                    if enabled {
                                        "❏ always on top"
                                    } else {
                                        "always on top off"
                                    }
                                    .into(),
                                    std::time::Instant::now(),
                                ));
                            }
                            Err(e) => this.message = e.to_string(),
                        }
                        cx.notify();
                        cx.stop_propagation();
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(
                    |this, _, window, _| match crate::native::WindowDrag::begin(window) {
                        Ok(drag) => this.drag = Some(drag),
                        Err(error) => this.message = error.to_string(),
                    },
                ),
            )
            .on_mouse_move(cx.listener(|this, _, _, _| {
                if !crate::native::right_button_down() {
                    this.drag = None;
                    return;
                }
                if let Some(drag) = this.drag.as_ref()
                    && let Err(error) = drag.update()
                {
                    this.message = error.to_string();
                }
            }))
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, _, _, _| this.drag = None),
            )
            .on_mouse_up_out(
                MouseButton::Right,
                cx.listener(|this, _, _, _| this.drag = None),
            )
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                if event.modifiers.alt && !this.alt_pressed {
                    this.alt_used = event.modifiers.control;
                }
                if !event.modifiers.alt && this.alt_pressed && !this.alt_used {
                    this.menu = !this.menu;
                    cx.notify();
                }
                this.alt_pressed = event.modifiers.alt;
            }))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, window, cx| {
                let scroll = event.delta.pixel_delta(px(24.));
                if crate::native::right_button_down() && scroll.x.abs() > scroll.y.abs() {
                    if this.width_motion.is_none() {
                        this.width_motion = Some(crate::window_motion::WidthMotion::new(
                            f32::from(window.bounds().size.width),
                        ));
                        this.width_tick = std::time::Instant::now();
                    }
                    if let Some(motion) = this.width_motion.as_mut() {
                        motion.scroll(f32::from(scroll.x));
                    }
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if event.modifiers.control {
                    let delta = f32::from(event.delta.pixel_delta(px(24.)).y);
                    if crate::native::right_button_down() {
                        this.drag = None;
                        this.appearance.opacity =
                            (this.appearance.opacity + delta / 1200.).clamp(0., 1.);
                        if let Err(e) = crate::native::set_opacity(window, this.appearance.opacity)
                        {
                            this.message = e.to_string();
                        }
                        let mut visual = cx.global::<Appearance>().clone();
                        visual.opacity = this.appearance.opacity;
                        cx.set_global(visual);
                    } else {
                        let step = if event.modifiers.shift { 0.001 } else { 0.25 };
                        let target = ((this.zoom_target.unwrap_or(this.appearance.font_size)
                            + if event.modifiers.shift {
                                delta.signum() * step
                            } else {
                                delta / 24. * step
                            })
                            * 1000.)
                            .round()
                            / 1000.;
                        let target = target.clamp(4., 128.);
                        this.appearance.font_size = target;
                        this.zoom_target = Some(target);
                        this.zoom_tick = std::time::Instant::now();
                    }
                    this.persist(cx);
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                if crate::native::right_button_down() {
                    let delta = event.delta.pixel_delta(px(24.));
                    if delta.y != px(0.) {
                        this.switch(delta.y < px(0.), window, cx);
                        cx.stop_propagation();
                    }
                }
            }))
            .when(self.menu, |root| {
                root.child(
                    div()
                        .flex()
                        .h(px(32.))
                        .items_center()
                        .gap_4()
                        .px_3()
                        .child(div().child("New").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                this.new_tab(cx);
                                this.focus(window, cx);
                                this.persist(cx);
                                cx.notify();
                            }),
                        ))
                        .child(div().child("Open").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| this.open(window, cx)),
                        ))
                        .child(div().child("Save").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| this.save(false, cx)),
                        ))
                        .child(div().child("[V] View").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.view_menu = !this.view_menu;
                                cx.notify();
                            }),
                        ))
                        .child(div().child("Language").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                let languages = [
                                    "txt", "rs", "py", "js", "ts", "json", "html", "css", "md",
                                    "c", "cpp", "sh", "yaml",
                                ];
                                this.tabs[this.active].editor.update(cx, |editor, cx| {
                                    let index = languages
                                        .iter()
                                        .position(|s| *s == editor.syntax)
                                        .unwrap_or(0);
                                    editor.syntax = languages[(index + 1) % languages.len()].into();
                                    cx.notify();
                                });
                            }),
                        ))
                        .child(
                            div()
                                .child("Move")
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, |_, window, _| {
                                    window.start_window_move()
                                }),
                        )
                        .child(div().child("Quit").cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.persist(cx);
                                cx.quit();
                            }),
                        )),
                )
            })
            .when(self.menu && self.view_menu, |root| {
                root.child(
                    div()
                        .flex()
                        .gap_4()
                        .px_3()
                        .h(px(30.))
                        .items_center()
                        .bg(rgb(theme.surface))
                        .child(
                            div()
                                .cursor_pointer()
                                .child(if appearance.show_tabs {
                                    "[T] Tabs ✓"
                                } else {
                                    "[T] Tabs"
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| this.toggle_tabs(window, cx)),
                                ),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .child(if appearance.word_wrap {
                                    "[W] Word Wrap ✓"
                                } else {
                                    "[W] Word Wrap"
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| this.toggle_wrap(window, cx)),
                                ),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .child(if self.unicode.is_some() {
                                    "[U] Unicode ✓"
                                } else {
                                    "[U] Unicode"
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| {
                                        this.toggle_unicode(window, cx)
                                    }),
                                ),
                        ),
                )
            })
            .when(appearance.show_tabs, |root| {
                root.child(
                    div()
                        .id("tabs")
                        .flex()
                        .flex_shrink_0()
                        .h(px(32.))
                        .overflow_x_scroll()
                        .items_center()
                        .px_2()
                        .gap_3()
                        .children(self.tabs.iter().enumerate().map(|(index, tab)| {
                            div()
                                .id(("tab", index))
                                .flex_shrink_0()
                                .px_2()
                                .py_1()
                                .cursor_pointer()
                                .text_color(rgb(if index == self.active {
                                    theme.foreground
                                } else {
                                    theme.muted
                                }))
                                .child(tab.title(index + 1, cx))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, window, cx| {
                                        this.active = index;
                                        this.focus(window, cx);
                                        this.persist(cx);
                                        cx.notify();
                                    }),
                                )
                                .on_mouse_down(
                                    MouseButton::Middle,
                                    cx.listener(move |this, _, window, cx| {
                                        if crate::native::right_button_down() {
                                            return;
                                        }
                                        this.active = index;
                                        this.close_tab(window, cx);
                                    }),
                                )
                        })),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(div().flex_1().min_w_0().px_3().pb_2().child(editor))
                    .children(unicode),
            )
            .when(self.search, |root| {
                root.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .flex_shrink_0()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child("Find")
                                .child(div().flex_1().h(px(26.)).child(self.query.clone()))
                                .child(div().child("Next ↵").cursor_pointer().on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| this.find(false, window, cx)),
                                )),
                        )
                        .when(self.replace, |bar| {
                            bar.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child("Replace")
                                    .child(
                                        div().flex_1().h(px(26.)).child(self.replacement.clone()),
                                    )
                                    .child(div().child("One").cursor_pointer().on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, window, cx| {
                                            this.replace_matches(false, window, cx)
                                        }),
                                    ))
                                    .child(div().child("All").cursor_pointer().on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, window, cx| {
                                            this.replace_matches(true, window, cx)
                                        }),
                                    )),
                            )
                        }),
                )
            })
            .when(self.menu || !self.message.is_empty(), |root| {
                root.child(
                    div()
                        .px_3()
                        .h(px(24.))
                        .text_color(rgb(0x777777))
                        .child(format!(
                            "{}   {}",
                            self.tabs[self.active].editor.read(cx).syntax,
                            self.message
                        )),
                )
            })
            .when_some(toast, |root, (text, alpha)| {
                root.child(
                    div()
                        .absolute()
                        .top(px(7.))
                        .right(px(10.))
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .bg(rgb(theme.surface).opacity(alpha))
                        .text_color(rgb(theme.foreground).opacity(alpha))
                        .child(text),
                )
            })
            .children(palette)
    }
}
