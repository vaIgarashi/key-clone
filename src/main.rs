mod hotkey;

use crate::hotkey::HotKey;
use log::{debug, info, warn};
use std::ffi::OsString;
use std::mem::{size_of, MaybeUninit};
use std::os::windows::ffi::OsStringExt;
use std::ptr::null_mut;
use std::time::Duration;
use std::{ptr, thread};
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, LRESULT, TRUE, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::handleapi::CloseHandle;
use winapi::um::tlhelp32::PROCESSENTRY32W;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use winapi::um::winuser::{
    CallNextHookEx, EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, SetWindowsHookExW,
    UnhookWindowsHookEx, VK_F7, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use winapi::um::winuser::{PostMessageW, KBDLLHOOKSTRUCT};

fn main() {
    simple_logger::init().expect("Failed to init logger");

    let mut run_hot_key = HotKey::new(1, VK_F7);
    run_hot_key.register();

    info!("Key clone are ready to run. Press \"F7\" to start");
    let mut hook_opt = None;

    loop {
        run_hot_key.process(|activated| {
            if activated {
                let hook = unsafe {
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), null_mut(), 0)
                };

                hook_opt = Some(hook);
                info!("Key clone started");
            } else {
                if let Some(hook) = hook_opt {
                    unsafe {
                        UnhookWindowsHookEx(hook);
                    }
                }

                info!("Key clone stopped");
            }
        });

        thread::sleep(Duration::from_millis(10));
    }
}

unsafe extern "system" fn hook_callback(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let hook_struct = *(l_param as *const KBDLLHOOKSTRUCT);
    let key = hook_struct.vkCode;
    let w_param_u32 = w_param as u32;

    if code >= 0 && w_param_u32 == WM_KEYDOWN
        || w_param_u32 == WM_KEYUP
        || w_param_u32 == WM_SYSKEYDOWN
        || w_param_u32 == WM_SYSKEYUP
    {
        let main_hwnd = GetForegroundWindow();
        let mut main_pid: u32 = 0;

        GetWindowThreadProcessId(main_hwnd, &mut main_pid);

        if let Some(name) = find_process_name(main_pid) {
            debug!(
                "Current foreground process \"{}\" (pid: {})",
                name, main_pid
            );

            let pids = find_processes_ids_by_name(&name);

            for pid in pids {
                if pid != main_pid {
                    if let Some(hwnd) = find_hwnd_by_pid(pid) {
                        debug!("Cloning key {} msg {} click to pid {}", key, w_param, pid);

                        if PostMessageW(hwnd, w_param as u32, key as usize, 0) == FALSE {
                            warn!("Failed to send key clone")
                        }
                    }
                }
            }
        }
    }

    CallNextHookEx(null_mut(), code, w_param, l_param)
}

unsafe fn find_hwnd_by_pid(pid: u32) -> Option<HWND> {
    static mut HWND_OPT: Option<HWND> = None;

    unsafe extern "system" fn enum_callback(hwnd: HWND, l_param: LPARAM) -> BOOL {
        let mut maybe_hwnd_pid = MaybeUninit::<u32>::uninit();

        GetWindowThreadProcessId(hwnd, maybe_hwnd_pid.as_mut_ptr());

        let hwnd_pid = maybe_hwnd_pid.assume_init();

        if hwnd_pid == l_param as u32 {
            HWND_OPT = Some(hwnd);
        }

        TRUE
    }

    EnumWindows(Some(enum_callback), pid as isize);

    HWND_OPT
}

unsafe fn find_process_name(pid: u32) -> Option<String> {
    let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

    let mut maybe_entry = MaybeUninit::<PROCESSENTRY32W>::uninit();

    ptr::write(
        &mut (*maybe_entry.as_mut_ptr()).dwSize,
        size_of::<PROCESSENTRY32W>() as u32,
    );

    let mut result = None;

    if Process32FirstW(handle, maybe_entry.as_mut_ptr()) == TRUE {
        while Process32NextW(handle, maybe_entry.as_mut_ptr()) == TRUE {
            let entry = maybe_entry.assume_init();

            if entry.th32ProcessID == pid {
                let process_name_full = &entry.szExeFile;
                let process_name_length = process_name_full.iter().take_while(|&&c| c != 0).count();

                let process_name = OsString::from_wide(&process_name_full[..process_name_length])
                    .into_string()
                    .expect("Failed to get process name");

                result = Some(process_name);
                break;
            }
        }
    }

    CloseHandle(handle);

    result
}

unsafe fn find_processes_ids_by_name(name: &str) -> Vec<u32> {
    let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    let mut pids = Vec::new();

    let mut maybe_entry = MaybeUninit::<PROCESSENTRY32W>::uninit();

    ptr::write(
        &mut (*maybe_entry.as_mut_ptr()).dwSize,
        size_of::<PROCESSENTRY32W>() as u32,
    );

    if Process32FirstW(handle, maybe_entry.as_mut_ptr()) == TRUE {
        while Process32NextW(handle, maybe_entry.as_mut_ptr()) == TRUE {
            let entry = maybe_entry.assume_init();

            let process_name_full = &entry.szExeFile;
            let process_name_length = process_name_full.iter().take_while(|&&c| c != 0).count();
            let process_name = &OsString::from_wide(&process_name_full[..process_name_length]);

            if process_name != name {
                continue;
            }

            pids.push(entry.th32ProcessID);
        }
    }

    CloseHandle(handle);

    pids
}
