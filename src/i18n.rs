//! Minimal two-language support (Traditional Chinese / English).
//!
//! The current language is thread-local: the GUI, CLI and exporters all run on one
//! thread, and tests running in parallel threads cannot disturb each other.
//! Strings are written inline as pairs: `t("中文", "English")` or
//! `tf!("{n} 個", "{n} items")`.

use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    /// 繁體中文 (default)
    #[default]
    #[serde(rename = "zh-TW", alias = "zh", alias = "zh_TW")]
    Zh,
    #[serde(rename = "en")]
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Zh, Lang::En];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh-TW",
            Lang::En => "en",
        }
    }

    /// Name of the language in itself (for the language menu).
    pub fn native(self) -> &'static str {
        match self {
            Lang::Zh => "繁體中文",
            Lang::En => "English",
        }
    }

    pub fn parse(s: &str) -> Result<Lang, String> {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "zh" | "zh-tw" | "zh-hant" | "tw" | "cht" => Ok(Lang::Zh),
            "en" | "en-us" | "en-gb" | "english" => Ok(Lang::En),
            other => Err(format!("unknown language {other:?} (use en or zh-TW)")),
        }
    }
}

thread_local! {
    static LANG: Cell<Lang> = const { Cell::new(Lang::Zh) };
}

pub fn lang() -> Lang {
    LANG.with(|l| l.get())
}

pub fn set_lang(l: Lang) {
    LANG.with(|c| c.set(l));
}

pub fn is_en() -> bool {
    lang() == Lang::En
}

/// Pick the string for the current language.
pub fn t(zh: &'static str, en: &'static str) -> &'static str {
    if is_en() { en } else { zh }
}

/// Run `f` with a temporary language (restored afterwards).
pub fn with_lang<R>(l: Lang, f: impl FnOnce() -> R) -> R {
    let old = lang();
    set_lang(l);
    let r = f();
    set_lang(old);
    r
}

/// `format!` with one format string per language: `tf!("{} 個", "{} items", n)`.
#[macro_export]
macro_rules! tf {
    ($zh:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::is_en() { format!($en $(, $arg)*) } else { format!($zh $(, $arg)*) }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch() {
        let n = 3;
        assert_eq!(tf!("{n} 個", "{n} items"), "3 個");
        with_lang(Lang::En, || {
            assert_eq!(t("是", "yes"), "yes");
            assert_eq!(tf!("{n} 個", "{n} items"), "3 items");
        });
        assert_eq!(lang(), Lang::Zh);
        assert_eq!(Lang::parse("EN").unwrap(), Lang::En);
        assert_eq!(serde_json::to_string(&Lang::Zh).unwrap(), "\"zh-TW\"");
    }
}
