use anyhow::Result;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::CreateMutexW,
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON},
            WindowsAndMessaging::*,
        },
    },
    core::{PCWSTR, w},
};

pub fn right_button_down() -> bool {
    unsafe { GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0 }
}
pub struct WindowDrag {
    window: windows::Win32::Foundation::HWND,
    pointer: windows::Win32::Foundation::POINT,
    bounds: windows::Win32::Foundation::RECT,
}
impl WindowDrag {
    pub fn begin(window: &gpui::Window) -> Result<Self> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .as_raw()
        else {
            anyhow::bail!("Windows window unavailable");
        };
        let mut drag = Self {
            window: windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut _),
            pointer: Default::default(),
            bounds: Default::default(),
        };
        unsafe {
            GetCursorPos(&mut drag.pointer)?;
            GetWindowRect(drag.window, &mut drag.bounds)?;
            windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(drag.window);
        }
        Ok(drag)
    }
    pub fn update(&self) -> Result<()> {
        let mut pointer = windows::Win32::Foundation::POINT::default();
        unsafe {
            GetCursorPos(&mut pointer)?;
            let dx = pointer.x - self.pointer.x;
            let dy = pointer.y - self.pointer.y;
            if dx.abs() + dy.abs() < 4 {
                return Ok(());
            }
            SetWindowPos(
                self.window,
                None,
                self.bounds.left + dx,
                self.bounds.top + dy,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
}
impl Drop for WindowDrag {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture() {
                eprintln!("Release mouse capture: {error}");
            }
        }
    }
}
pub fn remove_border(window: &gpui::Window) -> Result<()> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DWMWA_BORDER_COLOR, DwmSetWindowAttribute},
    };
    if let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("{error}"))?
        .as_raw()
    {
        let color: u32 = 0xfffffffe;
        unsafe {
            DwmSetWindowAttribute(
                HWND(handle.hwnd.get() as *mut _),
                DWMWA_BORDER_COLOR,
                &color as *const _ as *const _,
                std::mem::size_of_val(&color) as u32,
            )?;
        }
    }
    Ok(())
}
pub fn message(text: &str) {
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("winY"),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire(launcher: bool) -> Result<Option<Self>> {
        let mut name = if launcher {
            "Local\\winY-launcher".to_owned()
        } else {
            "Local\\winY-editor".to_owned()
        };
        if let Some(directory) = std::env::var_os("WINY_DATA_DIR") {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            directory.hash(&mut hash);
            name.push_str(&format!("-{:x}", hash.finish()));
        }
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let handle = CreateMutexW(None, false, PCWSTR(name.as_ptr()))?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle)?;
                return Ok(None);
            }
            Ok(Some(Self(handle)))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = CloseHandle(self.0) {
                eprintln!("Mutex: {error}");
            }
        }
    }
}
pub fn activate_existing() -> bool {
    unsafe {
        if let Ok(window) = FindWindowW(None, w!("winY")) {
            let _ = ShowWindow(window, SW_RESTORE);
            let _ = SetForegroundWindow(window);
            return true;
        }
    }
    false
}

fn hwnd(window: &gpui::Window) -> Result<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .as_raw()
    {
        RawWindowHandle::Win32(h) => Ok(windows::Win32::Foundation::HWND(h.hwnd.get() as *mut _)),
        _ => anyhow::bail!("Windows window unavailable"),
    }
}
pub fn set_opacity(window: &mut gpui::Window, opacity: f32) -> Result<()> {
    window.set_background_appearance(if opacity < 1. {
        gpui::WindowBackgroundAppearance::Transparent
    } else {
        gpui::WindowBackgroundAppearance::Opaque
    });
    Ok(())
}
pub fn set_topmost(window: &gpui::Window, enabled: bool) -> Result<()> {
    unsafe {
        SetWindowPos(
            hwnd(window)?,
            Some(if enabled {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

pub fn set_icon(window: &gpui::Window) -> Result<()> {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    let hwnd = hwnd(window)?;
    unsafe {
        let instance = GetModuleHandleW(None)?;
        for (kind, size) in [(ICON_SMALL, 32), (ICON_BIG, 64)] {
            let icon = LoadImageW(
                Some(instance.into()),
                PCWSTR(std::ptr::without_provenance(1)),
                IMAGE_ICON,
                size,
                size,
                LR_SHARED,
            )?;
            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(windows::Win32::Foundation::WPARAM(kind as usize)),
                Some(windows::Win32::Foundation::LPARAM(icon.0 as isize)),
            );
        }
    }
    Ok(())
}
