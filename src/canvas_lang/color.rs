//! Color type and parsing for the canvas drawing language.
//!
//! Supports `#RRGGBB` hex colors and named colors (red, green, blue, etc.)
//! with bright_ variants.

/// An RGB color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CanvasColor {
    /// Creates a new `CanvasColor` from RGB components.
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Formats this color as a `{#RRGGBB}` tag string.
    pub fn to_tag(self) -> String {
        format!("{{#{:02X}{:02X}{:02X}}}", self.r, self.g, self.b)
    }
}

/// Parses a color string into a `CanvasColor`.
///
/// Accepts:
/// - `#RRGGBB` hex format (case-insensitive)
/// - Named colors: red, green, blue, yellow, cyan, magenta, white, gray/grey
/// - Bright variants: bright_red, bright_green, bright_blue, bright_yellow,
///   bright_cyan, bright_magenta, bright_white, bright_gray/bright_grey
pub fn parse_color(input: &str) -> Option<CanvasColor> {
    let trimmed = input.trim();

    // Try hex format
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(CanvasColor::new(r, g, b));
        }
        return None;
    }

    // Named colors (case-insensitive)
    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        "red" => Some(CanvasColor::new(205, 0, 0)),
        "green" => Some(CanvasColor::new(0, 205, 0)),
        "blue" => Some(CanvasColor::new(0, 0, 238)),
        "yellow" => Some(CanvasColor::new(205, 205, 0)),
        "cyan" => Some(CanvasColor::new(0, 205, 205)),
        "magenta" => Some(CanvasColor::new(205, 0, 205)),
        "white" => Some(CanvasColor::new(229, 229, 229)),
        "gray" | "grey" => Some(CanvasColor::new(128, 128, 128)),

        "bright_red" => Some(CanvasColor::new(255, 0, 0)),
        "bright_green" => Some(CanvasColor::new(0, 255, 0)),
        "bright_blue" => Some(CanvasColor::new(92, 92, 255)),
        "bright_yellow" => Some(CanvasColor::new(255, 255, 0)),
        "bright_cyan" => Some(CanvasColor::new(0, 255, 255)),
        "bright_magenta" => Some(CanvasColor::new(255, 0, 255)),
        "bright_white" => Some(CanvasColor::new(255, 255, 255)),
        "bright_gray" | "bright_grey" => Some(CanvasColor::new(192, 192, 192)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_parsing_valid() {
        let c = parse_color("#FF8800").unwrap();
        assert_eq!(c, CanvasColor::new(255, 136, 0));
    }

    #[test]
    fn test_hex_parsing_lowercase() {
        let c = parse_color("#ff8800").unwrap();
        assert_eq!(c, CanvasColor::new(255, 136, 0));
    }

    #[test]
    fn test_hex_parsing_mixed_case() {
        let c = parse_color("#Ff8800").unwrap();
        assert_eq!(c, CanvasColor::new(255, 136, 0));
    }

    #[test]
    fn test_hex_invalid_too_short() {
        assert!(parse_color("#FFF").is_none());
    }

    #[test]
    fn test_hex_invalid_not_hex() {
        assert!(parse_color("#GGHHII").is_none());
    }

    #[test]
    fn test_hex_invalid_too_long() {
        assert!(parse_color("#FF880000").is_none());
    }

    #[test]
    fn test_named_red() {
        let c = parse_color("red").unwrap();
        assert_eq!(c, CanvasColor::new(205, 0, 0));
    }

    #[test]
    fn test_named_green() {
        let c = parse_color("green").unwrap();
        assert_eq!(c, CanvasColor::new(0, 205, 0));
    }

    #[test]
    fn test_named_blue() {
        let c = parse_color("blue").unwrap();
        assert_eq!(c, CanvasColor::new(0, 0, 238));
    }

    #[test]
    fn test_named_gray_grey() {
        let g1 = parse_color("gray").unwrap();
        let g2 = parse_color("grey").unwrap();
        assert_eq!(g1, g2);
    }

    #[test]
    fn test_named_bright_gray_grey() {
        let g1 = parse_color("bright_gray").unwrap();
        let g2 = parse_color("bright_grey").unwrap();
        assert_eq!(g1, g2);
    }

    #[test]
    fn test_case_insensitivity() {
        let c1 = parse_color("RED").unwrap();
        let c2 = parse_color("Red").unwrap();
        let c3 = parse_color("red").unwrap();
        assert_eq!(c1, c2);
        assert_eq!(c2, c3);
    }

    #[test]
    fn test_to_tag() {
        let c = CanvasColor::new(255, 136, 0);
        assert_eq!(c.to_tag(), "{#FF8800}");
    }

    #[test]
    fn test_to_tag_zero_padded() {
        let c = CanvasColor::new(0, 10, 0);
        assert_eq!(c.to_tag(), "{#000A00}");
    }

    #[test]
    fn test_unknown_color_returns_none() {
        assert!(parse_color("chartreuse").is_none());
    }

    #[test]
    fn test_bright_variants() {
        assert!(parse_color("bright_red").is_some());
        assert!(parse_color("bright_green").is_some());
        assert!(parse_color("bright_blue").is_some());
        assert!(parse_color("bright_yellow").is_some());
        assert!(parse_color("bright_cyan").is_some());
        assert!(parse_color("bright_magenta").is_some());
        assert!(parse_color("bright_white").is_some());
    }
}
