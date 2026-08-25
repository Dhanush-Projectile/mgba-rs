//! Safe Rust wrapper around [libmgba](https://mgba.io/) for GBA emulation.

use std::ffi::CString;
use std::marker::PhantomData;
use std::path::Path;

pub const GBA_WIDTH: usize = 240;
pub const GBA_HEIGHT: usize = 160;
pub const GBA_PIXELS: usize = GBA_WIDTH * GBA_HEIGHT;

/// Resamples the emulator's native-rate stereo i16 output to an arbitrary
/// hardware sample rate, producing f32 samples in the -1.0..=1.0 range.
pub struct AudioResampler {
    step: f64,
    frac: f64,
    last: [f32; 2],
}

impl AudioResampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        Self {
            step: f64::from(input_rate) / f64::from(output_rate),
            frac: 0.0,
            last: [0.0, 0.0],
        }
    }

    pub fn set_input_rate(&mut self, input_rate: u32, output_rate: u32) {
        self.step = f64::from(input_rate) / f64::from(output_rate);
    }

    /// Push a chunk of interleaved stereo i16 samples and call `emit` for
    /// each resampled f32 output sample (left, right, left, right, ...).
    pub fn push(&mut self, input: &[i16], mut emit: impl FnMut(f32)) {
        for [left_in, right_in] in input.as_chunks::<2>().0 {
            let cur = [*left_in as f32 / 32768.0, *right_in as f32 / 32768.0];
            while self.frac < 1.0 {
                let t = self.frac as f32;
                let left = self.last[0] + (cur[0] - self.last[0]) * t;
                let right = self.last[1] + (cur[1] - self.last[1]) * t;
                emit(left.clamp(-1.0, 1.0));
                emit(right.clamp(-1.0, 1.0));
                self.frac += self.step;
            }
            self.frac -= 1.0;
            self.last = cur;
        }
    }
}

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
            // GBACoreCreate zeroes opts; without this masterVolume ends up 0
            // (silence) since no config file provides a volume value.
            mgba_sys::wrapper_mCoreSetOptionVolume(raw, 0x100);
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

            // Keep battery saves (.sav) next to the ROM file, overriding any
            // savegamePath from an ambient mGBA config.
            if let Some(parent) = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .and_then(|p| p.to_str())
            {
                if let Ok(c_dir) = CString::new(parent) {
                    mgba_sys::wrapper_mCoreSetSaveDirectory(self.raw, c_dir.as_ptr());
                }
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

    pub fn audio_sample_rate(&self) -> u32 {
        unsafe { mgba_sys::wrapper_mCoreAudioSampleRate(self.raw) }
    }

    pub fn bus_read16(&mut self, address: u32) -> u16 {
        unsafe { mgba_sys::wrapper_mCoreBusRead16(self.raw, address) as u16 }
    }

    /// Drains all available audio from the emulator, resamples it to
    /// `output_rate`, and calls `emit` for each f32 stereo sample.
    ///
    /// Use an [`AudioResampler`] stored between frames to maintain
    /// interpolation state.
    pub fn drain_audio(
        &mut self,
        resampler: &mut AudioResampler,
        output_rate: u32,
        emit: &mut dyn FnMut(f32),
    ) {
        resampler.set_input_rate(self.audio_sample_rate(), output_rate);
        let mut buf = [0i16; 4096];
        loop {
            let n = self.read_audio(&mut buf);
            if n == 0 {
                break;
            }
            resampler.push(&buf[..n], &mut *emit);
        }
    }

    /// Drains interleaved stereo i16 samples into `out`, returning how many
    /// values were written (always a multiple of 2).
    pub fn read_audio(&mut self, out: &mut [i16]) -> usize {
        if out.len() < 2 {
            return 0;
        }
        unsafe {
            let buffer = mgba_sys::wrapper_mCoreGetAudioBuffer(self.raw);
            if buffer.is_null() {
                return 0;
            }
            let frames = (out.len() / 2) as usize;
            let read = mgba_sys::mAudioBufferRead(buffer, out.as_mut_ptr(), frames);
            read * 2
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                // wrapper_mCoreDeinit -> core->deinit already frees the core.
                mgba_sys::wrapper_mCoreDeinit(self.raw);
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
