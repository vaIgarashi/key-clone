mod hotkey;

use crate::hotkey::HotKey;
use log::{debug, info};
use std::ffi::OsString;
use std::mem::{size_of, MaybeUninit};
use std::os::windows::ffi::OsStringExt;
use std::time::Duration;
use std::{ptr, thread};
use winapi::shared::minwindef::TRUE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::tlhelp32::PROCESSENTRY32W;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use winapi::um::winuser::{GetForegroundWindow, GetWindowThreadProcessId, VK_F7};

fn main() {
    simple_logger::init().expect("Failed to init logger");

    info!("Started key clone");

    let mut run_hot_key = HotKey::new(1, VK_F7);
    run_hot_key.register();

    let mut run = false;

    loop {
        run_hot_key.process(|activated| {
            if activated {
                run = true;
            } else {
                run = false;
            }
        });

        if run {
            unsafe { process() }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

unsafe fn process() {
    let hwnd = GetForegroundWindow();
    let mut pid: u32 = 0;

    GetWindowThreadProcessId(hwnd, &mut pid);

    if let Some(name) = find_process_name(pid) {
        debug!("Current foreground process \"{}\" (pid: {})", name, pid);
    }
}

unsafe fn find_process_name(pid: u32) -> Option<String> {
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    let mut maybe_entry = MaybeUninit::<PROCESSENTRY32W>::uninit();

    unsafe {
        ptr::write(
            &mut (*maybe_entry.as_mut_ptr()).dwSize,
            size_of::<PROCESSENTRY32W>() as u32,
        );
    }

    let mut result = None;

    if unsafe { Process32FirstW(handle, maybe_entry.as_mut_ptr()) } == TRUE {
        while unsafe { Process32NextW(handle, maybe_entry.as_mut_ptr()) } == TRUE {
            let entry = unsafe { maybe_entry.assume_init() };

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

    unsafe {
        CloseHandle(handle);
    }

    result
}
