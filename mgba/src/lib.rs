//! Safe Rust wrapper around [libmgba](https://mgba.io/) for GBA emulation.

use std::ffi::CString;
use std::marker::PhantomData;
use std::path::Path;

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

pub const GBA_WIDTH: usize = 240;
pub const GBA_HEIGHT: usize = 160;
pub const GBA_PIXELS: usize = GBA_WIDTH * GBA_HEIGHT;

pub struct Core {
    raw: *mut mgba_sys::mCore,
    video_buffer: Box<[u32; GBA_PIXELS]>,
    display_buffer: Box<[u32; GBA_PIXELS]>,
    loaded: bool,
    _not_sync: PhantomData<*const ()>,
}

unsafe impl Send for Core {}

impl Core {
    pub fn new() -> Result<Self, CoreError> {
        unsafe {
            let raw = mgba_sys::GBACoreCreate();
            if raw.is_null() {
                return Err(CoreError::CreateFailed);
            }

            mgba_sys::wrapper_mCoreInit(raw);
            mgba_sys::mCoreInitConfig(raw, std::ptr::null());
            mgba_sys::mCoreLoadConfig(raw);

            Ok(Core {
                raw,
                video_buffer: Box::new([0u32; GBA_PIXELS]),
                display_buffer: Box::new([0u32; GBA_PIXELS]),
                loaded: false,
                _not_sync: PhantomData,
            })
        }
    }

    pub fn load_rom(&mut self, path: &Path) -> Result<(), CoreError> {
        let path_str = path.to_str().ok_or(CoreError::InvalidPath)?;
        let c_path = CString::new(path_str).map_err(|_| CoreError::InvalidPath)?;

        unsafe {
            mgba_sys::wrapper_mCoreSetVideoBuffer(self.raw, self.video_buffer.as_mut_ptr(), GBA_WIDTH);

            if !mgba_sys::mCoreLoadFile(self.raw, c_path.as_ptr()) {
                return Err(CoreError::RomLoadFailed);
            }
            mgba_sys::mCoreAutoloadSave(self.raw);
        }

        self.loaded = true;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), CoreError> {
        unsafe { mgba_sys::wrapper_mCoreReset(self.raw); }
        Ok(())
    }

    pub fn run_frame(&mut self) -> Result<(), CoreError> {
        unsafe { mgba_sys::wrapper_mCoreRunFrame(self.raw); }
        for (src, dst) in self.video_buffer.iter().zip(self.display_buffer.iter_mut()) {
            *dst = (src & 0xFF00FF00) | ((src & 0x00FF0000) >> 16) | ((src & 0x000000FF) << 16);
        }
        Ok(())
    }

    pub fn set_keys(&mut self, keys: u32) -> Result<(), CoreError> {
        unsafe { mgba_sys::wrapper_mCoreSetKeys(self.raw, keys); }
        Ok(())
    }

    pub fn video_buffer(&self) -> &[u32; GBA_PIXELS] {
        &self.display_buffer
    }

}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                mgba_sys::wrapper_mCoreDeinit(self.raw);
                free(self.raw as *mut std::ffi::c_void);
            }
        }
    }
}

#[derive(Debug)]
pub enum CoreError {
    CreateFailed,
    InvalidPath,
    RomLoadFailed,
}
