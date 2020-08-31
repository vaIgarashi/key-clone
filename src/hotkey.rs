use log::debug;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use winapi::um::winuser::{PeekMessageW, RegisterHotKey, UnregisterHotKey, PM_NOREMOVE};
use winapi::um::winuser::{MSG, PM_REMOVE, WM_HOTKEY};

pub struct HotKey {
    pub id: i32,
    pub key: i32,
    pub registered: bool,
    pub activated: bool,
}

impl HotKey {
    pub fn new(id: i32, key: i32) -> HotKey {
        HotKey {
            id,
            key,
            registered: false,
            activated: false,
        }
    }

    pub fn register(&mut self) -> bool {
        if self.registered {
            return true;
        }

        let result = unsafe { RegisterHotKey(null_mut(), self.id, 0, self.key as u32) };

        if result == 1 {
            debug!(
                "Registered global hook for virtual key {} with id {}",
                self.key, self.id
            );
            self.registered = true;
            true
        } else {
            false
        }
    }

    pub fn process<F: FnMut(bool)>(&mut self, mut f: F) {
        let mut maybe_msg = MaybeUninit::<MSG>::uninit();

        let result = unsafe {
            PeekMessageW(
                maybe_msg.as_mut_ptr(),
                null_mut(),
                WM_HOTKEY,
                WM_HOTKEY,
                PM_NOREMOVE,
            )
        };

        if result == 0 {
            return;
        }

        let msg = unsafe { maybe_msg.assume_init() };

        if msg.wParam == self.id as usize {
            unsafe {
                PeekMessageW(
                    maybe_msg.as_mut_ptr(),
                    null_mut(),
                    WM_HOTKEY,
                    WM_HOTKEY,
                    PM_REMOVE,
                )
            };

            self.activated = !self.activated;

            f(self.activated)
        }
    }

    pub fn unregister(&mut self) -> bool {
        if !self.registered {
            return true;
        }

        let result = unsafe { UnregisterHotKey(null_mut(), self.key as i32) };

        if result == 1 {
            debug!(
                "Unregistered global hook for key {} with id {}",
                self.key, self.id
            );

            self.registered = false;
            true
        } else {
            false
        }
    }
}

impl Drop for HotKey {
    fn drop(&mut self) {
        self.unregister();
    }
}
