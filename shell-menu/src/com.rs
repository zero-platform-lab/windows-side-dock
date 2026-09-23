//! エクスプローラーから呼ばれるCOMの部分。OSのシェルに読み込まれて初めて動くため、
//! 自動テストの対象外。判断を伴う処理は `command_line.rs` に置く。

use crate::command_line::split_command;
use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use windows::core::{implement, w, Interface, Ref, Result, BOOL, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_NOTIMPL, E_POINTER, HINSTANCE, HMODULE,
    MAX_PATH, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{IBindCtx, IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl,
    IShellItemArray, SHStrDupW, ShellExecuteW, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_ENABLED,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// スパースパッケージのマニフェストに書くクラスID。変えるとマニフェストも直す必要がある。
pub const CLSID_WINDOWS_SIDE_DOCK_MENU: GUID =
    GUID::from_u128(0x23e04cb3_838f_4762_b46f_9aefaa12eb5b);

/// 従来メニューの「プロセスツール」項目。Dock（`src/shell_menu.rs`）が設定に合わせて書き換える。
const PROCESS_TOOL_KEY: PCWSTR =
    w!(r"Software\Classes\Directory\Background\Shell\WindowsSideDock\shell\02ProcessTool");
const PROCESS_TOOL_COMMAND_KEY: PCWSTR =
    w!(r"Software\Classes\Directory\Background\Shell\WindowsSideDock\shell\02ProcessTool\command");

static MODULE: AtomicIsize = AtomicIsize::new(0);
static LIVE_OBJECTS: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "system" fn DllMain(module: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(module.0 as isize, Ordering::Relaxed);
    }
    BOOL(1)
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LIVE_OBJECTS.load(Ordering::Relaxed) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// # Safety
/// COMの規約どおり、有効なポインターで呼ばれること。
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if class_id.is_null() || interface_id.is_null() || object.is_null() {
        return E_POINTER;
    }
    *object = std::ptr::null_mut();
    if *class_id != CLSID_WINDOWS_SIDE_DOCK_MENU {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = Factory.into();
    factory.query(interface_id, object)
}

/// COMオブジェクトの生存数を数え、使われている間はDLLを解放させない。
struct Alive;

impl Alive {
    fn new() -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for Alive {
    fn drop(&mut self) {
        LIVE_OBJECTS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<windows::core::IUnknown>,
        interface_id: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if object.is_null() || interface_id.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe { *object = std::ptr::null_mut() };
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let command: IExplorerCommand = MenuCommand::new(Item::Root).into();
        unsafe { command.query(interface_id, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Item {
    /// 「Windows Side Dock」。下の2つをサブメニューに持つ。
    Root,
    OpenFolder,
    ProcessTool,
}

#[implement(IExplorerCommand)]
struct MenuCommand {
    item: Item,
    _alive: Alive,
}

impl MenuCommand {
    fn new(item: Item) -> Self {
        Self {
            item,
            _alive: Alive::new(),
        }
    }
}

fn duplicate(text: &str) -> Result<PWSTR> {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    unsafe { SHStrDupW(PCWSTR(wide.as_ptr())) }
}

/// このDLLのあるフォルダー（Windows Side Dockのインストール先）。
fn install_directory() -> Option<std::path::PathBuf> {
    let mut buffer = vec![0_u16; MAX_PATH as usize * 4];
    let module = HMODULE(MODULE.load(Ordering::Relaxed) as *mut c_void);
    let length = unsafe { GetModuleFileNameW(Some(module), &mut buffer) } as usize;
    if length == 0 {
        return None;
    }
    let path = std::path::PathBuf::from(String::from_utf16_lossy(&buffer[..length]));
    path.parent().map(std::path::Path::to_path_buf)
}

/// `HKEY_CURRENT_USER\key` の文字列値。`name` が `None` なら既定値。
fn read_registry(key: PCWSTR, name: Option<PCWSTR>) -> Option<String> {
    let mut size = 0_u32;
    let name = name.unwrap_or(PCWSTR::null());
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .ok()
        .ok()?;
    }
    let mut buffer = vec![0_u16; size as usize / 2 + 1];
    let mut size = (buffer.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            name,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .ok()
        .ok()?;
    }
    let length = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..length]))
}

fn process_tool() -> (String, String) {
    let label = read_registry(PROCESS_TOOL_KEY, Some(w!("MUIVerb")))
        .unwrap_or_else(|| "タスク マネージャー".to_owned());
    let command =
        read_registry(PROCESS_TOOL_COMMAND_KEY, None).unwrap_or_else(|| "taskmgr.exe".to_owned());
    (label, command)
}

fn shell_open(file: &str, parameters: &str) {
    let file: Vec<u16> = file.encode_utf16().chain(Some(0)).collect();
    let parameters: Vec<u16> = parameters.encode_utf16().chain(Some(0)).collect();
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(file.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

impl IExplorerCommand_Impl for MenuCommand_Impl {
    fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        match self.item {
            Item::Root => duplicate("Windows Side Dock"),
            Item::OpenFolder => duplicate("Windows Side Dockの場所を開く"),
            Item::ProcessTool => duplicate(&process_tool().0),
        }
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        let executable = install_directory()
            .map(|directory| directory.join("windows-side-dock.exe"))
            .ok_or_else(|| windows::core::Error::from(E_NOTIMPL))?;
        duplicate(&format!("{},0", executable.display()))
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        Err(E_NOTIMPL.into())
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(GUID::zeroed())
    }

    fn GetState(&self, _items: Ref<IShellItemArray>, _slow_ok: BOOL) -> Result<u32> {
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(&self, _items: Ref<IShellItemArray>, _context: Ref<IBindCtx>) -> Result<()> {
        match self.item {
            Item::Root => {}
            Item::OpenFolder => {
                if let Some(directory) = install_directory() {
                    shell_open("explorer.exe", &format!("\"{}\"", directory.display()));
                }
            }
            Item::ProcessTool => {
                let (file, parameters) = split_command(&process_tool().1);
                shell_open(&file, &parameters);
            }
        }
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(match self.item {
            Item::Root => ECF_HASSUBCOMMANDS.0 as u32,
            _ => ECF_DEFAULT.0 as u32,
        })
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        match self.item {
            Item::Root => Ok(SubCommands::new(vec![Item::OpenFolder, Item::ProcessTool], 0).into()),
            _ => Err(E_NOTIMPL.into()),
        }
    }
}

#[implement(IEnumExplorerCommand)]
struct SubCommands {
    items: Vec<Item>,
    next: AtomicUsize,
    _alive: Alive,
}

impl SubCommands {
    fn new(items: Vec<Item>, next: usize) -> Self {
        Self {
            items,
            next: AtomicUsize::new(next),
            _alive: Alive::new(),
        }
    }
}

impl IEnumExplorerCommand_Impl for SubCommands_Impl {
    fn Next(
        &self,
        count: u32,
        commands: *mut Option<IExplorerCommand>,
        fetched: *mut u32,
    ) -> HRESULT {
        if commands.is_null() {
            return E_POINTER;
        }
        let mut written = 0_u32;
        while written < count {
            let index = self.next.load(Ordering::Relaxed);
            let Some(&item) = self.items.get(index) else {
                break;
            };
            let command: IExplorerCommand = MenuCommand::new(item).into();
            unsafe { commands.add(written as usize).write(Some(command)) };
            self.next.store(index + 1, Ordering::Relaxed);
            written += 1;
        }
        if !fetched.is_null() {
            unsafe { *fetched = written };
        }
        if written == count {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn Skip(&self, count: u32) -> Result<()> {
        let next = self.next.load(Ordering::Relaxed) + count as usize;
        self.next
            .store(next.min(self.items.len()), Ordering::Relaxed);
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        self.next.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        Ok(SubCommands::new(self.items.clone(), self.next.load(Ordering::Relaxed)).into())
    }
}
