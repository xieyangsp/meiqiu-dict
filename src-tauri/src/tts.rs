use windows::core::HSTRING;
use windows::Win32::Media::Speech::{ISpVoice, SpVoice, SPF_DEFAULT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

use crate::error::{AppError, AppResult};

struct ComApartment;

impl ComApartment {
    unsafe fn init() -> AppResult<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|e| AppError::Other(format!("tts COM init: {e}")))?;
        }
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

pub fn speak(text: &str) -> AppResult<()> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Other("tts text is empty".into()));
    }

    unsafe {
        let _com = ComApartment::init()?;
        let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
            .map_err(|e| AppError::Other(format!("tts voice create: {e}")))?;
        let text = HSTRING::from(text);
        let mut stream_number = 0u32;
        voice
            .Speak(
                &text,
                SPF_DEFAULT.0 as u32,
                Some(&mut stream_number),
            )
            .map_err(|e| AppError::Other(format!("tts speak: {e}")))?;
    }

    Ok(())
}