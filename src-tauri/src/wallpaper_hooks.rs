//! Brief §8: re-attach after Explorer restarts (TaskbarCreated) or WorkerW destroy.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::{Arc, OnceLock};

    use crate::wallpaper_subclass;
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, GetClassNameW, RegisterWindowMessageW,
    };

    const EVENT_OBJECT_DESTROY: u32 = 0x8001;
    const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;

    const HOOK_SUBCLASS_ID: usize = 0x4c47_0002;
    static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);
    static REATTACH: OnceLock<Arc<dyn Fn(&str) + Send + Sync>> = OnceLock::new();
    static WIN_EVENT_HOOK_PTR: AtomicUsize = AtomicUsize::new(0);

    pub fn set_reattach_handler(handler: Arc<dyn Fn(&str) + Send + Sync>) {
        let _ = REATTACH.set(handler);
    }

    pub fn install(main_hwnd: HWND, _app: tauri::AppHandle) {
        unsafe {
            let msg = RegisterWindowMessageW(w!("TaskbarCreated"));
            TASKBAR_CREATED_MSG.store(msg, Ordering::Relaxed);
            let _ = wallpaper_subclass::set_subclass(
                main_hwnd,
                Some(main_hook_subclass),
                HOOK_SUBCLASS_ID,
            );
        }
        install_workerw_destroy_hook();
    }

    fn install_workerw_destroy_hook() {
        unsafe extern "system" fn on_destroy(
            _hook: HWINEVENTHOOK,
            _event: u32,
            hwnd: HWND,
            _id_object: i32,
            _id_child: i32,
            _thread: u32,
            _time: u32,
        ) {
            if hwnd.0.is_null() {
                return;
            }
            let mut class = [0u16; 32];
            let len = unsafe { GetClassNameW(hwnd, &mut class) };
            if len == 0 {
                return;
            }
            let name = String::from_utf16_lossy(&class[..len as usize]);
            if name == "WorkerW" {
                schedule_reattach("winevent-workerw-destroy");
            }
        }

        unsafe {
            let hook = SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_DESTROY,
                None,
                Some(on_destroy),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.0.is_null() {
                return;
            }
            WIN_EVENT_HOOK_PTR.store(hook.0 as usize, Ordering::Relaxed);
        }
    }

    pub fn drop_hooks(main_hwnd: HWND) {
        unsafe {
            let ptr = WIN_EVENT_HOOK_PTR.swap(0, Ordering::Relaxed);
            if ptr != 0 {
                let _ = UnhookWinEvent(HWINEVENTHOOK(ptr as _));
            }
            wallpaper_subclass::remove_subclass(
                main_hwnd,
                Some(main_hook_subclass),
                HOOK_SUBCLASS_ID,
            );
        }
    }

    unsafe extern "system" fn main_hook_subclass(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        let taskbar = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
        if taskbar != 0 && msg == taskbar {
            schedule_reattach("taskbar-created");
        }
        wallpaper_subclass::def_subclass_proc(hwnd, msg, wparam, lparam)
    }

    fn schedule_reattach(trigger: &'static str) {
        if let Some(handler) = REATTACH.get() {
            handler(trigger);
        }
    }

    pub fn workerw_present() -> bool {
        unsafe {
            let progman = FindWindowW(w!("Progman"), None);
            if progman.is_err() {
                return false;
            }
            FindWindowExW(Some(progman.unwrap()), None, w!("WorkerW"), None).is_ok()
        }
    }
}

#[cfg(windows)]
pub use imp::{drop_hooks, install, set_reattach_handler, workerw_present};

#[cfg(not(windows))]
pub fn set_reattach_handler(_: std::sync::Arc<dyn Fn(&str) + Send + Sync>) {}

#[cfg(not(windows))]
pub fn install(_: (), _: tauri::AppHandle) {}

#[cfg(not(windows))]
pub fn drop_hooks(_: ()) {}