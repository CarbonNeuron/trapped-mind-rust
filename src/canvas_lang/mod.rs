//! Canvas drawing language — parses a simple script into drawing commands
//! and renders them onto a text canvas with color tags.

pub mod color;
pub mod font;
pub mod parser;
pub mod primitives;
pub mod renderer;

/// Parses a canvas-lang script and renders it to colored text lines.
///
/// Returns `None` if the input contains no valid commands.
pub fn parse_and_render(input: &str, width: usize, height: usize) -> Option<Vec<String>> {
    let commands = parser::parse_script(input);
    if commands.is_empty() {
        return None;
    }
    let mut canvas = renderer::Canvas::new(width, height);
    canvas.execute_all(&commands);
    Some(canvas.to_lines())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_render_basic() {
        let script = "FILL .\nTEXT 0,0,\"HI\"";
        let lines = parse_and_render(script, 10, 3).unwrap();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_parse_and_render_empty_returns_none() {
        assert!(parse_and_render("just some garbage", 10, 3).is_none());
    }

    #[test]
    fn test_parse_and_render_blank_returns_none() {
        assert!(parse_and_render("", 10, 3).is_none());
    }

    #[test]
    fn test_parse_and_render_with_colors() {
        let script = "FILL . #1a1a2e\nTEXT 0,0,\"HI\" #FF0000";
        let lines = parse_and_render(script, 10, 3).unwrap();
        assert_eq!(lines.len(), 3);
        // Should contain hex color tags
        let all = lines.join("");
        assert!(all.contains("{#"));
    }
}
