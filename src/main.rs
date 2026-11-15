#![windows_subsystem = "windows"]
mod app;
mod appearance;
mod caret;
mod csv;
mod document;
mod editor;
mod hotkey;
mod native;
mod session;
mod todo;
mod window_motion;
mod wrap;

use gpui::*;
use std::{borrow::Cow, rc::Rc};

fn main() {
    if let Err(error) = run() {
        native::message(&format!("{error:#}"));
    }
}
fn run() -> anyhow::Result<()> {
    if std::env::args().any(|arg| arg == "--benchmark-buffer") {
        let text = "0123456789abcdef0123456789abcdef\n".repeat(338_250);
        let mut document = document::Document::new(&text);
        let mut samples = Vec::with_capacity(1_000);
        for index in 0..1_000 {
            let at = document.text.len_chars() / 2 + index % 100;
            let start = std::time::Instant::now();
            document.replace(at..at, "x");
            std::hint::black_box(&document);
            samples.push(start.elapsed().as_nanos());
        }
        samples.sort_unstable();
        println!(
            "{{\"buffer_bytes\":{},\"edits\":1000,\"median_ns\":{},\"p95_ns\":{},\"max_ns\":{},\"scope\":\"rope edit plus undo snapshot; excludes UI and disk\"}}",
            text.len(),
            samples[500],
            samples[950],
            samples[999]
        );
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--hotkey-launcher") {
        return hotkey::run();
    }
    let Some(_instance) = native::Instance::acquire(false)? else {
        native::activate_existing();
        return Ok(());
    };
    let (manifest, texts) = session::load()?;
    let platform = Rc::new(gpui_windows::WindowsPlatform::new(false)?);
    Application::with_platform(platform).run(move |cx| {
        let mut appearance = manifest.appearance.clone();
        appearance.normalize();
        cx.set_global(appearance);
        if let Err(error) = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../upstream/assets/fonts/lilex/Lilex-Regular.ttf"
            ))])
        {
            native::message(&format!("Could not load font: {error}"));
            cx.quit();
            return;
        }
        let bounds = Bounds::centered(None, size(px(1000.), px(700.)), cx);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                window_min_size: Some(size(px(10.), px(10.))),
                inactive_frame_interval: None,
                low_power_frame_interval: None,
                thermal_frame_interval: None,
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("winY");
                if manifest.appearance.opacity < 1.
                    && let Err(error) = native::set_opacity(window, manifest.appearance.opacity)
                {
                    eprintln!("Window opacity: {error}");
                }
                if let Err(error) = native::set_icon(window) {
                    eprintln!("Window icon: {error}");
                }
                if let Err(error) = native::remove_border(window) {
                    eprintln!("Window border: {error}");
                }
                cx.new(|cx| app::WinY::new(manifest, texts, cx))
            },
        );
        match result {
            Ok(window) => {
                if let Err(error) = window.update(cx, |view, window, cx| {
                    view.focus(window, cx);
                    cx.activate(true);
                }) {
                    native::message(&error.to_string());
                }
            }
            Err(error) => {
                native::message(&format!("Could not open window: {error}"));
                cx.quit();
            }
        }
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
    Ok(())
}
