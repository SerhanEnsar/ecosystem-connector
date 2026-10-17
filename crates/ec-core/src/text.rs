/// Longest device name accepted, in characters.
pub const MAX_DEVICE_NAME_CHARS: usize = 64;

/// Cleans a device name received from another device before it is stored or shown.
///
/// Control characters are removed (they could rewrite a terminal or spoof UI text),
/// whitespace is collapsed and the result is cut to [`MAX_DEVICE_NAME_CHARS`].
/// An empty result becomes `"Unnamed device"`.
pub fn sanitize_device_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control() && !is_bidi_control(*c))
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = collapsed.chars().take(MAX_DEVICE_NAME_CHARS).collect();
    if truncated.is_empty() {
        "Unnamed device".to_owned()
    } else {
        truncated
    }
}

/// Unicode bidirectional overrides can make text render in a misleading order.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_control_and_bidi_characters() {
        assert_eq!(
            sanitize_device_name("Pixel\u{1b}[2J 8\u{202E}"),
            "Pixel[2J 8"
        );
    }

    #[test]
    fn collapses_whitespace_and_truncates() {
        assert_eq!(sanitize_device_name("  My \n\t Mac  "), "My Mac");
        assert_eq!(sanitize_device_name(&"a".repeat(200)).chars().count(), 64);
    }

    #[test]
    fn empty_name_gets_placeholder() {
        assert_eq!(sanitize_device_name("\u{7}\u{0}"), "Unnamed device");
    }
}
