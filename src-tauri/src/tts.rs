use windows::core::HSTRING;
use windows::Win32::Media::Speech::{ISpVoice, SpVoice, SPF_DEFAULT, SPF_IS_XML};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub enum Accent {
    EnUs,
    EnGb,
}

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

pub fn speak_with_accent(text: &str, accent: Accent) -> AppResult<()> {
    let plain = text.trim();
    if plain.is_empty() {
        return Err(AppError::Other("tts text is empty".into()));
    }

    let lang = match accent {
        Accent::EnUs => "en-US",
        Accent::EnGb => "en-GB",
    };
    let escaped = escape_xml(plain);
    let ssml = format!("<speak version=\"1.0\" xml:lang=\"{lang}\">{escaped}</speak>");

    unsafe {
        let _com = ComApartment::init()?;
        let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
            .map_err(|e| AppError::Other(format!("tts voice create: {e}")))?;
        let text = HSTRING::from(ssml);
        let mut stream_number = 0u32;
        match voice.Speak(&text, SPF_IS_XML.0 as u32, Some(&mut stream_number)) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::warn!("tts SSML speak failed, falling back to default voice: {e}");
                // Fallback to plain speak if the current voice rejects SSML.
                speak(plain)
            }
        }
    }
}

fn escape_xml(text: &str) -> String {
    text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}