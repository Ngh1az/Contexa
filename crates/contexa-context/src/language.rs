//! Language Detector — `docs/06_Context_Engine.md` §5.6.

const MIN_CHARS_FOR_DETECTION: usize = 20;

#[must_use]
pub fn detect_language(text: &str) -> Option<String> {
    if text.len() < MIN_CHARS_FOR_DETECTION {
        return None;
    }
    let info = whatlang::detect(text)?;
    Some(info.lang().code().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_not_detected() {
        assert_eq!(detect_language("hi"), None);
    }

    #[test]
    fn long_english_text_is_detected() {
        let text = "The quick brown fox jumps over the lazy dog repeatedly.";
        assert_eq!(detect_language(text).as_deref(), Some("eng"));
    }
}
