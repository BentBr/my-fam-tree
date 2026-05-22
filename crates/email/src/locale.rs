//! User-facing language for outbound email templates.
//!
//! Kept minimal and additive: callers default to English when the requested
//! locale is unknown rather than failing, so a stale or misconfigured user
//! preference never blocks a transactional email.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    De,
}

impl Locale {
    /// Parse `"en"` / `"de"` (case-insensitive). Falls back to [`Locale::En`]
    /// for anything else.
    #[must_use]
    pub const fn from_str_or_en(s: &str) -> Self {
        if s.as_bytes().eq_ignore_ascii_case(b"de") { Self::De } else { Self::En }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_de_case_insensitively() {
        assert_eq!(Locale::from_str_or_en("de"), Locale::De);
        assert_eq!(Locale::from_str_or_en("DE"), Locale::De);
        assert_eq!(Locale::from_str_or_en("De"), Locale::De);
    }

    #[test]
    fn defaults_to_en_for_unknown() {
        assert_eq!(Locale::from_str_or_en("en"), Locale::En);
        assert_eq!(Locale::from_str_or_en("fr"), Locale::En);
        assert_eq!(Locale::from_str_or_en(""), Locale::En);
    }
}
