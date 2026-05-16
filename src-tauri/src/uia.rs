use std::cell::OnceCell;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
};
use windows::core::Interface;

use crate::selection::SelectionOutcome;

// Guards against providers that return whole-document content when nothing is selected.
const MAX_TEXT: i32 = 4096;

thread_local! {
    // None means COM init failed; we log once and never retry on this thread.
    static UIA: OnceCell<Option<IUIAutomation>> = const { OnceCell::new() };
}

pub fn try_get_selection() -> SelectionOutcome {
    UIA.with(|cell| {
        let uia = cell.get_or_init(init_uia);
        match uia {
            Some(uia) => query_focused_selection(uia),
            None => SelectionOutcome::Unsupported,
        }
    })
}

fn init_uia() -> Option<IUIAutomation> {

    // COINIT_MULTITHREADED is the apartment UIA clients want; RPC_E_CHANGED_MODE
    // means another component picked a different one and we just adopt it.
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() && hr.0 != windows::Win32::Foundation::RPC_E_CHANGED_MODE.0 {
            log::warn!("UIA CoInitializeEx failed: {hr:?}");
            return None;
        }
    }
    match unsafe {
        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
    } {
        Ok(uia) => Some(uia),
        Err(e) => {
            log::warn!("UIA CoCreateInstance failed: {e:?}");
            None
        }
    }
}

fn query_focused_selection(uia: &IUIAutomation) -> SelectionOutcome {
    let element = match unsafe { uia.GetFocusedElement() } {
        Ok(el) => el,
        Err(e) => {
            log::debug!("UIA GetFocusedElement failed: {e:?}");
            return SelectionOutcome::Unsupported;
        }
    };

    let pattern_unknown = match unsafe { element.GetCurrentPattern(UIA_TextPatternId) } {
        Ok(p) => p,
        Err(e) => {
            log::debug!("UIA GetCurrentPattern(TextPatternId) failed: {e:?}");
            return SelectionOutcome::Unsupported;
        }
    };
    let text_pattern: IUIAutomationTextPattern = match pattern_unknown.cast() {
        Ok(p) => p,
        Err(e) => {
            log::debug!("UIA cast to IUIAutomationTextPattern failed: {e:?}");
            return SelectionOutcome::Unsupported;
        }
    };

    let ranges = match unsafe { text_pattern.GetSelection() } {
        Ok(r) => r,
        Err(e) => {
            log::debug!("UIA GetSelection failed: {e:?}");
            return SelectionOutcome::Unsupported;
        }
    };

    let len = match unsafe { ranges.Length() } {
        Ok(n) => n,
        Err(e) => {
            log::debug!("UIA TextRangeArray Length failed: {e:?}");
            return SelectionOutcome::Unsupported;
        }
    };
    if len == 0 {
        return SelectionOutcome::NoSelection;
    }

    let mut acc = String::new();
    for i in 0..len {
        if acc.len() as i32 >= MAX_TEXT {
            break;
        }
        let range = match unsafe { ranges.GetElement(i) } {
            Ok(r) => r,
            Err(e) => {
                log::debug!("UIA TextRangeArray GetElement({i}) failed: {e:?}");
                continue;
            }
        };
        match unsafe { range.GetText(MAX_TEXT) } {
            Ok(bstr) => acc.push_str(&bstr.to_string()),
            Err(e) => log::debug!("UIA TextRange GetText failed: {e:?}"),
        }
    }

    if acc.trim().is_empty() {
        SelectionOutcome::NoSelection
    } else {
        if acc.len() > MAX_TEXT as usize {
            acc.truncate(MAX_TEXT as usize);
        }
        SelectionOutcome::Text(acc)
    }
}
