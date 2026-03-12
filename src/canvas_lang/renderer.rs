//! 2D cell buffer renderer for the canvas drawing language.

use crate::canvas_lang::parser::DrawCommand;

pub struct Canvas;

impl Canvas {
    pub fn new(_w: usize, _h: usize) -> Self { Canvas }
    pub fn execute_all(&mut self, _commands: &[DrawCommand]) {}
    pub fn to_lines(&self) -> Vec<String> { vec![] }
}
