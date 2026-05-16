use std::path::PathBuf;

const BUSINESS: &[&str] = &["capture", "dict", "hotkey", "tray", "tts"];
const INFRASTRUCTURE: &[&str] = &["state", "error", "config", "events"];
const UTILITY: &[&str] = &["selection", "window"];

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read_module(name: &str) -> Option<String> {
    let path = src_dir().join(format!("{name}.rs"));
    std::fs::read_to_string(path).ok()
}

fn assert_no_import(src: &str, src_content: &str, forbidden: &str, reason: &str) {
    let needle = format!("use crate::{forbidden}");
    assert!(
        !src_content.contains(&needle),
        "tier violation: {src}.rs imports {forbidden} ({reason}). \
         See AGENTS.md > Architecture."
    );
}

#[test]
fn business_modules_do_not_import_each_other() {
    for src in BUSINESS {
        let Some(content) = read_module(src) else {
            continue;
        };
        for other in BUSINESS {
            if src == other {
                continue;
            }
            assert_no_import(src, &content, other, "business -> business forbidden");
        }
    }
}

#[test]
fn infrastructure_does_not_import_business() {
    for src in INFRASTRUCTURE {
        let Some(content) = read_module(src) else {
            continue;
        };
        for biz in BUSINESS {
            assert_no_import(src, &content, biz, "infrastructure -> business forbidden");
        }
    }
}

#[test]
fn utility_does_not_import_business_or_infrastructure_state() {
    for src in UTILITY {
        let Some(content) = read_module(src) else {
            continue;
        };
        for biz in BUSINESS {
            assert_no_import(src, &content, biz, "utility must stay pure");
        }
        assert_no_import(src, &content, "state", "utility must stay pure (no AppState)");
    }
}

#[test]
fn no_module_imports_commands() {
    let src = src_dir();
    for entry in std::fs::read_dir(&src).expect("read src dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if name == "commands" || name == "lib" || name == "main" {
            continue;
        }
        let content = std::fs::read_to_string(&path).expect("read");
        assert_no_import(name, &content, "commands", "IPC boundary; nothing imports commands");
    }
}
