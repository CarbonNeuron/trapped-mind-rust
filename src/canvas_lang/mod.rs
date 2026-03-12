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
