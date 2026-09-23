//! 開発用: ビルドしたDLLをエクスプローラーと同じ手順で読み込み、メニューの項目を表示する。
//! `cargo run -p windows-side-dock-shell --example probe -- <DLLのパス>`

#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use windows::core::{Interface, GUID, HSTRING, PCSTR};
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::Com::{CoInitializeEx, IClassFactory, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::UI::Shell::{IEnumExplorerCommand, IExplorerCommand};
    use windows_side_dock_shell::CLSID_WINDOWS_SIDE_DOCK_MENU;

    type GetClassObject = unsafe extern "system" fn(
        *const GUID,
        *const GUID,
        *mut *mut core::ffi::c_void,
    ) -> windows::core::HRESULT;

    let path = std::env::args().nth(1).expect("DLLのパスを渡す");
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let module: HMODULE = LoadLibraryW(&HSTRING::from(path))?;
        let entry = GetProcAddress(module, PCSTR(c"DllGetClassObject".as_ptr().cast()))
            .expect("DllGetClassObject がない");
        let get_class_object: GetClassObject = std::mem::transmute(entry);
        let mut factory = std::ptr::null_mut();
        get_class_object(
            &CLSID_WINDOWS_SIDE_DOCK_MENU,
            &IClassFactory::IID,
            &mut factory,
        )
        .ok()?;
        let factory = IClassFactory::from_raw(factory);
        let root: IExplorerCommand = factory.CreateInstance(None)?;
        let show = |command: &IExplorerCommand, indent: &str| -> windows::core::Result<()> {
            let title = command.GetTitle(None)?;
            let icon = command
                .GetIcon(None)
                .map(|icon| icon.to_string().unwrap_or_default());
            println!(
                "{indent}{} flags={} icon={:?}",
                title.to_string().unwrap_or_default(),
                command.GetFlags()?,
                icon.ok()
            );
            Ok(())
        };
        show(&root, "")?;
        let children: IEnumExplorerCommand = root.EnumSubCommands()?;
        loop {
            let mut item = [None];
            let mut fetched = 0;
            let _ = children.Next(&mut item, Some(&mut fetched));
            if fetched == 0 {
                break;
            }
            show(item[0].as_ref().unwrap(), "  - ")?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
