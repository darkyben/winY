use anyhow::{Context, Result};
use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, HWND, LPARAM, WPARAM},
        System::{Com::CoTaskMemFree, Registry::*},
        UI::{
            Shell::{FOLDERID_Startup, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
            WindowsAndMessaging::*,
        },
    },
    core::w,
};
const STOP: u32 = WM_APP + 71;
const RUN: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
fn control_window() -> Option<HWND> {
    unsafe {
        FindWindowExW(
            Some(HWND_MESSAGE),
            None,
            w!("STATIC"),
            w!("winY hotkey control"),
        )
        .ok()
    }
}
pub fn active() -> bool {
    control_window().is_some()
}
fn legacy_shortcut() -> Result<PathBuf> {
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Startup, KF_FLAG_DEFAULT, None)?;
        let value = path.to_string();
        CoTaskMemFree(Some(path.0.cast()));
        Ok(PathBuf::from(value?).join("winY Hotkey.lnk"))
    }
}
pub fn startup_enabled() -> bool {
    (unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            w!("winY"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
        .is_ok()
    }) || legacy_shortcut().is_ok_and(|p| p.is_file())
}
fn set_startup(enabled: bool) -> Result<()> {
    // Migrate the shortcut created by winY's installer to one owned Run entry.
    if let Ok(path) = legacy_shortcut() {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    unsafe {
        if enabled {
            let command = format!(
                "\"{}\" --hotkey-launcher",
                std::env::current_exe()?.display()
            );
            let data: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN,
                w!("winY"),
                REG_SZ.0,
                Some(data.as_ptr().cast()),
                (data.len() * 2) as u32,
            )
            .ok()?;
        } else {
            let status = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, w!("winY"));
            if status != ERROR_FILE_NOT_FOUND {
                status.ok()?;
            }
        }
    }
    Ok(())
}
pub fn configure(action: &str) -> Result<String> {
    match action {
        "off" => {
            set_startup(false)?;
            if let Some(hwnd) = control_window() {
                unsafe {
                    PostMessageW(Some(hwnd), STOP, WPARAM(0), LPARAM(0))?;
                }
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while active() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            anyhow::ensure!(!active(), "The Win+Y launcher did not stop");
            Ok("Win+Y disabled; Windows startup disabled".into())
        }
        "no-startup" => {
            set_startup(false)?;
            Ok("Windows startup disabled; current Win+Y state unchanged".into())
        }
        "now" | "startup" => {
            if !active() {
                let mut child = Command::new(std::env::current_exe()?)
                    .arg("--hotkey-launcher")
                    .creation_flags(0x08000000)
                    .spawn()?;
                let deadline = Instant::now() + Duration::from_secs(3);
                while !active() && Instant::now() < deadline {
                    if child.try_wait()?.is_some() {
                        anyhow::bail!(
                            "Could not enable Win+Y. Another application may already own this shortcut."
                        );
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                anyhow::ensure!(
                    active(),
                    "Could not enable Win+Y; check whether another application uses it"
                );
            }
            if action == "startup" {
                set_startup(true)?;
            }
            Ok(if action == "startup" {
                "Win+Y enabled; starts with Windows"
            } else {
                "Win+Y enabled"
            }
            .into())
        }
        _ => anyhow::bail!("Unknown hotkey option"),
    }
}
pub fn run() -> Result<()> {
    let Some(_instance) = crate::native::Instance::acquire(true)? else {
        return Ok(());
    };
    unsafe {
        windows::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey(
            None,
            1,
            windows::Win32::UI::Input::KeyboardAndMouse::MOD_WIN
                | windows::Win32::UI::Input::KeyboardAndMouse::MOD_NOREPEAT,
            b'Y' as u32,
        )
        .context("Win+Y is already used by another application")?;
        let control = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("winY hotkey control"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )?;
        let result = (|| -> Result<()> {
            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, None, 0, 0).0;
                anyhow::ensure!(result != -1, "GetMessageW failed");
                if result == 0 || msg.message == STOP {
                    break;
                }
                if msg.message == WM_HOTKEY && !crate::native::activate_existing() {
                    Command::new(std::env::current_exe()?)
                        .creation_flags(0x08000000)
                        .spawn()?;
                }
            }
            Ok(())
        })();
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey(None, 1);
        let _ = DestroyWindow(control);
        result
    }
}
