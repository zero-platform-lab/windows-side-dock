use crate::model::{friendly_window_name, IconKind, LauncherItem, RunningWindow};
use std::path::Path;

#[cfg(windows)]
pub(crate) fn open_target(target: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    let target: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        ) as isize
            > 32
    }
}

#[cfg(not(windows))]
pub(crate) fn open_target(_target: &str) -> bool {
    false
}

#[cfg(windows)]
pub(crate) fn running_apps() -> Vec<LauncherItem> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 || GetWindowTextLengthW(hwnd) == 0 {
            return 1;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 || process_id == GetCurrentProcessId() {
            return 1;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return 1;
        }
        let mut path = vec![0_u16; 32768];
        let mut length = path.len() as u32;
        let success = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length);
        CloseHandle(process);
        if success == 0 || length == 0 {
            return 1;
        }
        let command = String::from_utf16_lossy(&path[..length as usize]);
        let title_length = GetWindowTextLengthW(hwnd);
        let mut title_buffer = vec![0_u16; title_length.max(0) as usize + 1];
        let copied = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        let title = if copied > 0 {
            String::from_utf16_lossy(&title_buffer[..copied as usize])
        } else {
            command.clone()
        };
        (&mut *(parameter as *mut Vec<(isize, String, String)>)).push((
            hwnd as isize,
            command,
            title,
        ));
        1
    }

    let mut windows = Vec::<(isize, String, String)>::new();
    unsafe {
        EnumWindows(Some(enumerate), &mut windows as *mut _ as LPARAM);
    }
    let foreground = unsafe { GetForegroundWindow() as isize };
    let mut items: Vec<LauncherItem> = Vec::new();
    for (window, command, title) in windows {
        if let Some(existing) = items
            .iter_mut()
            .find(|item| item.command.eq_ignore_ascii_case(&command))
        {
            existing.windows.push(RunningWindow {
                handle: window,
                title,
            });
            existing.active |= window == foreground;
        } else {
            let executable_name = Path::new(&command)
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("アプリ")
                .to_owned();
            items.push(LauncherItem {
                name: friendly_window_name(&title, &executable_name),
                icon: load_shell_icon(&command),
                command,
                fallback_icon: IconKind::File,
                windows: vec![RunningWindow {
                    handle: window,
                    title,
                }],
                active: window == foreground,
            });
        }
    }
    items
}

#[cfg(not(windows))]
pub(crate) fn running_apps() -> Vec<LauncherItem> {
    Vec::new()
}

#[cfg(windows)]
pub(crate) fn activate_taskbar_item(windows: &[RunningWindow]) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, SetForegroundWindow, ShowWindow, SW_MINIMIZE, SW_RESTORE,
    };
    if let Some(item) = windows.first() {
        unsafe {
            let window = item.handle as HWND;
            if GetForegroundWindow() == window {
                ShowWindow(window, SW_MINIMIZE);
            } else {
                if IsIconic(window) != 0 {
                    ShowWindow(window, SW_RESTORE);
                }
                SetForegroundWindow(window);
            }
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn activate_taskbar_item(_windows: &[RunningWindow]) {}

#[cfg(windows)]
pub(crate) fn close_all_windows(windows: &[RunningWindow]) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
    for item in windows {
        unsafe {
            PostMessageW(item.handle as HWND, WM_CLOSE, 0, 0);
        }
    }
}

#[cfg(not(windows))]
pub(crate) fn close_all_windows(_windows: &[RunningWindow]) {}

#[cfg(windows)]
pub(crate) fn load_shell_icon(path: &str) -> Option<eframe::egui::ColorImage> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::HINSTANCE;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};
    const SIZE: usize = 48;
    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let mut info = SHFILEINFOW::default();
    if unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    } == 0
        || info.hIcon.is_null()
    {
        return None;
    }
    let dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    if dc.is_null() {
        unsafe {
            DestroyIcon(info.hIcon);
        }
        return None;
    }
    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: SIZE as i32,
        biHeight: -(SIZE as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            dc,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut::<std::ffi::c_void>() as HINSTANCE,
            0,
        )
    };
    if bitmap.is_null() || bits.is_null() {
        unsafe {
            DeleteDC(dc);
            DestroyIcon(info.hIcon);
        }
        return None;
    }
    let old = unsafe { SelectObject(dc, bitmap) };
    unsafe {
        DrawIconEx(
            dc,
            0,
            0,
            info.hIcon,
            SIZE as i32,
            SIZE as i32,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        );
    }
    let bgra = unsafe { std::slice::from_raw_parts(bits as *const u8, SIZE * SIZE * 4) };
    let mut rgba = Vec::with_capacity(bgra.len());
    for p in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    unsafe {
        SelectObject(dc, old);
        DeleteObject(bitmap);
        DeleteDC(dc);
        DestroyIcon(info.hIcon);
    }
    Some(eframe::egui::ColorImage::from_rgba_unmultiplied(
        [SIZE, SIZE],
        &rgba,
    ))
}

#[cfg(not(windows))]
pub(crate) fn load_shell_icon(_path: &str) -> Option<eframe::egui::ColorImage> {
    None
}
