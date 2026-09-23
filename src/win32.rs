//! `Platform` のWin32実装。OSを実際に操作するだけの薄い層で、判断は `platform.rs` などに置く。
//! 実行中のプログラム起動・実ウィンドウ操作・レジストリ書き込み・モーダルダイアログを伴うため、
//! 自動テストとカバレッジ計測の対象外にしている（`main.rs` の `coverage(off)`）。

use crate::platform::{LocalTime, Platform};
use eframe::egui;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};

pub(crate) struct WindowsPlatform;

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn to_rect(rect: RECT) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(rect.left as f32, rect.top as f32),
        egui::pos2(rect.right as f32, rect.bottom as f32),
    )
}

impl Platform for WindowsPlatform {
    fn open_target(&self, target: &str) -> bool {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let operation = wide("open");
        let target = wide(target);
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

    fn visible_windows(&self) -> Vec<(isize, String, String)> {
        use windows_sys::core::BOOL;
        use windows_sys::Win32::Foundation::{CloseHandle, LPARAM};
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
            IsWindowVisible,
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
        windows
    }

    fn foreground_window(&self) -> isize {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        unsafe { GetForegroundWindow() as isize }
    }

    fn is_minimized(&self, window: isize) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::IsIconic;
        unsafe { IsIconic(window as HWND) != 0 }
    }

    fn minimize(&self, window: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
        unsafe { ShowWindow(window as HWND, SW_MINIMIZE) };
    }

    fn restore(&self, window: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};
        unsafe { ShowWindow(window as HWND, SW_RESTORE) };
    }

    fn set_foreground(&self, window: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        unsafe { SetForegroundWindow(window as HWND) };
    }

    fn close_window(&self, window: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
        unsafe { PostMessageW(window as HWND, WM_CLOSE, 0, 0) };
    }

    fn load_icon(&self, path: &str) -> Option<egui::ColorImage> {
        load_shell_icon(path)
    }

    fn cursor_position(&self) -> Option<egui::Pos2> {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut cursor = POINT::default();
        (unsafe { GetCursorPos(&mut cursor) } != 0)
            .then(|| egui::pos2(cursor.x as f32, cursor.y as f32))
    }

    fn dock_rect(&self) -> Option<egui::Rect> {
        let title = wide("Windows Side Dock");
        let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        let mut rect = RECT::default();
        (!window.is_null() && unsafe { GetWindowRect(window, &mut rect) } != 0)
            .then(|| to_rect(rect))
    }

    fn screen_width(&self) -> f32 {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};
        unsafe { GetSystemMetrics(SM_CXSCREEN) as f32 }
    }

    fn work_area(&self) -> Option<egui::Rect> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
        let mut area = RECT::default();
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                &mut area as *mut _ as *mut std::ffi::c_void,
                0,
            )
        };
        (ok != 0).then(|| to_rect(area))
    }

    fn local_time(&self) -> LocalTime {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut time = SYSTEMTIME::default();
        unsafe { GetLocalTime(&mut time) };
        LocalTime {
            month: time.wMonth,
            day: time.wDay,
            weekday: time.wDayOfWeek,
            hour: time.wHour,
            minute: time.wMinute,
        }
    }

    fn choose_executable(&self) -> Option<String> {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };
        let owner_title = wide("Windows Side Dock 設定");
        let dialog_title = wide("Process Explorerを選択");
        let filter: Vec<u16> = "実行ファイル (*.exe)\0*.exe\0すべてのファイル\0*.*\0\0"
            .encode_utf16()
            .collect();
        let mut file_buffer = vec![0_u16; 32768];
        let mut options = OPENFILENAMEW::default();
        options.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        options.hwndOwner = unsafe { FindWindowW(std::ptr::null(), owner_title.as_ptr()) };
        options.lpstrFilter = filter.as_ptr();
        options.lpstrFile = file_buffer.as_mut_ptr();
        options.nMaxFile = file_buffer.len() as u32;
        options.lpstrTitle = dialog_title.as_ptr();
        options.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
        if unsafe { GetOpenFileNameW(&mut options) } == 0 {
            return None;
        }
        let length = file_buffer.iter().position(|&value| value == 0)?;
        Some(String::from_utf16_lossy(&file_buffer[..length]))
    }

    fn install_directory(&self) -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
    }

    fn registry_key_exists(&self, key: &str) -> bool {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };
        let key = wide(key);
        let mut handle: HKEY = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_READ, &mut handle) } != 0
        {
            return false;
        }
        unsafe { RegCloseKey(handle) };
        true
    }

    fn set_registry_string(&self, key: &str, subkey: &str, name: &str, value: &str) {
        use windows_sys::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
        let path = if subkey.is_empty() {
            key.to_owned()
        } else {
            format!(r"{key}\{subkey}")
        };
        let path = wide(&path);
        let name = wide(name);
        let data: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                path.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            );
        }
    }
}

fn load_shell_icon(path: &str) -> Option<egui::ColorImage> {
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
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [SIZE, SIZE],
        &rgba,
    ))
}
