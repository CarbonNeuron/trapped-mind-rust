//! Canvas renderer — executes drawing commands onto a text grid.

/// A text canvas that drawing commands operate on.
pub struct Canvas;

impl Canvas {
    /// Creates a new canvas with the given dimensions.
    pub fn new(_w: usize, _h: usize) -> Self {
        Canvas
    }

    /// Executes all commands on the canvas in order.
    pub fn execute_all(&mut self, _commands: &[()]) {}

    /// Converts the canvas to a list of output lines with color tags.
    pub fn to_lines(&self) -> Vec<String> {
        vec![]
    }
}
