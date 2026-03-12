//! 2D cell buffer renderer for the canvas drawing language.

use std::time::Instant;

use crate::canvas_lang::color::CanvasColor;
use crate::canvas_lang::parser::DrawCommand;
use crate::canvas_lang::primitives;

/// A single cell in the canvas buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub color: Option<CanvasColor>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            color: None,
        }
    }
}

/// A 2D grid of cells that can be drawn on and serialized to color-tagged strings.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
}

impl Canvas {
    /// Creates a new canvas filled with default (space, no color) cells.
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Self {
            width,
            height,
            cells,
        }
    }

    /// Sets a cell at (x, y). Out-of-bounds coordinates are silently ignored.
    pub fn set(&mut self, x: i32, y: i32, ch: char, color: Option<CanvasColor>) {
        if x < 0 || y < 0 {
            return;
        }
        let ux = x as usize;
        let uy = y as usize;
        if ux < self.width && uy < self.height {
            self.cells[uy][ux] = Cell { ch, color };
        }
    }

    /// Gets a reference to the cell at (x, y), if in bounds.
    pub fn get(&self, x: usize, y: usize) -> Option<&Cell> {
        self.cells.get(y).and_then(|row| row.get(x))
    }

    /// Executes all commands with a 500ms timeout.
    pub fn execute_all(&mut self, commands: &[DrawCommand]) {
        let deadline = Instant::now() + std::time::Duration::from_millis(500);
        for cmd in commands {
            if Instant::now() >= deadline {
                break;
            }
            self.execute(cmd);
        }
    }

    /// Dispatches a single draw command.
    pub fn execute(&mut self, cmd: &DrawCommand) {
        match cmd {
            DrawCommand::Clear => {
                for row in &mut self.cells {
                    for cell in row.iter_mut() {
                        *cell = Cell::default();
                    }
                }
            }
            DrawCommand::Fill { ch, color } => {
                for row in &mut self.cells {
                    for cell in row.iter_mut() {
                        cell.ch = *ch;
                        cell.color = *color;
                    }
                }
            }
            DrawCommand::Text { x, y, text, color } => {
                for (i, ch) in text.chars().enumerate() {
                    self.set(*x + i as i32, *y, ch, *color);
                }
            }
            DrawCommand::HLine {
                y,
                x1,
                x2,
                ch,
                color,
            } => {
                let (sx, ex) = if x1 <= x2 { (*x1, *x2) } else { (*x2, *x1) };
                for x in sx..=ex {
                    self.set(x, *y, *ch, *color);
                }
            }
            DrawCommand::VLine {
                x,
                y1,
                y2,
                ch,
                color,
            } => {
                let (sy, ey) = if y1 <= y2 { (*y1, *y2) } else { (*y2, *y1) };
                for y in sy..=ey {
                    self.set(*x, y, *ch, *color);
                }
            }
            DrawCommand::Rect {
                x,
                y,
                w,
                h,
                ch,
                color,
            } => {
                primitives::filled_rect(self, *x, *y, *w, *h, *ch, *color);
            }
            DrawCommand::Outline {
                x,
                y,
                w,
                h,
                ch,
                color,
            } => {
                primitives::outline_rect(self, *x, *y, *w, *h, *ch, *color);
            }
            DrawCommand::Frame { x, y, w, h, color } => {
                primitives::frame_box(self, *x, *y, *w, *h, *color);
            }
            DrawCommand::RoundBox { x, y, w, h, color } => {
                primitives::round_box(self, *x, *y, *w, *h, *color);
            }
            DrawCommand::Line {
                x1,
                y1,
                x2,
                y2,
                ch,
                color,
            } => {
                primitives::bresenham_line(self, *x1, *y1, *x2, *y2, *ch, *color);
            }
            DrawCommand::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
            } => {
                primitives::arrow(self, *x1, *y1, *x2, *y2, *color);
            }
            DrawCommand::BoxLine {
                x1,
                y1,
                x2,
                y2,
                color,
            } => {
                primitives::box_line(self, *x1, *y1, *x2, *y2, *color);
            }
            DrawCommand::Circle {
                cx,
                cy,
                r,
                ch,
                color,
            } => {
                primitives::filled_circle(self, *cx, *cy, *r, *ch, *color);
            }
            DrawCommand::Ring {
                cx,
                cy,
                r,
                ch,
                color,
            } => {
                primitives::ring(self, *cx, *cy, *r, *ch, *color);
            }
            DrawCommand::Ellipse {
                cx,
                cy,
                rx,
                ry,
                ch,
                color,
            } => {
                primitives::filled_ellipse(self, *cx, *cy, *rx, *ry, *ch, *color);
            }
            DrawCommand::Tri {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
                ch,
                color,
            } => {
                primitives::filled_triangle(self, *x1, *y1, *x2, *y2, *x3, *y3, *ch, *color);
            }
            DrawCommand::Gradient {
                x,
                y,
                w,
                h,
                direction,
            } => {
                primitives::gradient(self, *x, *y, *w, *h, *direction);
            }
            DrawCommand::Pattern {
                x,
                y,
                w,
                h,
                pattern,
                color,
            } => {
                primitives::pattern(self, *x, *y, *w, *h, *pattern, *color);
            }
            DrawCommand::BigText {
                x,
                y,
                text,
                color,
            } => {
                primitives::big_text(self, *x, *y, text, *color);
            }
        }
    }

    /// Serializes the canvas to color-tagged strings, one per row.
    pub fn to_lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.height);
        for row in &self.cells {
            let mut line = String::new();
            let mut current_color: Option<CanvasColor> = None;

            for cell in row {
                if cell.color != current_color {
                    if current_color.is_some() {
                        line.push_str("{/}");
                    }
                    if let Some(c) = cell.color {
                        line.push_str(&c.to_tag());
                    }
                    current_color = cell.color;
                }
                line.push(cell.ch);
            }

            if current_color.is_some() {
                line.push_str("{/}");
            }
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_lang::parser::parse_script;

    #[test]
    fn canvas_new() {
        let c = Canvas::new(5, 3);
        assert_eq!(c.width, 5);
        assert_eq!(c.height, 3);
        assert_eq!(c.cells.len(), 3);
        assert_eq!(c.cells[0].len(), 5);
    }

    #[test]
    fn set_and_get() {
        let mut c = Canvas::new(10, 10);
        c.set(3, 4, 'X', None);
        let cell = c.get(3, 4).unwrap();
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.color, None);
    }

    #[test]
    fn set_out_of_bounds() {
        let mut c = Canvas::new(5, 5);
        // These should not panic
        c.set(-1, 0, 'X', None);
        c.set(0, -1, 'X', None);
        c.set(5, 0, 'X', None);
        c.set(0, 5, 'X', None);
        c.set(100, 100, 'X', None);
    }

    #[test]
    fn clear() {
        let mut c = Canvas::new(5, 5);
        c.set(2, 2, 'X', Some(CanvasColor::new(255, 0, 0)));
        c.execute(&DrawCommand::Clear);
        let cell = c.get(2, 2).unwrap();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.color, None);
    }

    #[test]
    fn fill() {
        let mut c = Canvas::new(3, 2);
        c.execute(&DrawCommand::Fill {
            ch: '.',
            color: None,
        });
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(c.get(x, y).unwrap().ch, '.');
            }
        }
    }

    #[test]
    fn text() {
        let mut c = Canvas::new(20, 5);
        c.execute(&DrawCommand::Text {
            x: 1,
            y: 0,
            text: "hi".to_string(),
            color: None,
        });
        assert_eq!(c.get(1, 0).unwrap().ch, 'h');
        assert_eq!(c.get(2, 0).unwrap().ch, 'i');
    }

    #[test]
    fn hline() {
        let mut c = Canvas::new(10, 5);
        c.execute(&DrawCommand::HLine {
            y: 2,
            x1: 1,
            x2: 5,
            ch: '-',
            color: None,
        });
        for x in 1..=5 {
            assert_eq!(c.get(x, 2).unwrap().ch, '-');
        }
        assert_eq!(c.get(0, 2).unwrap().ch, ' ');
    }

    #[test]
    fn serialize_no_color() {
        let mut c = Canvas::new(3, 1);
        c.set(0, 0, 'a', None);
        c.set(1, 0, 'b', None);
        c.set(2, 0, 'c', None);
        let lines = c.to_lines();
        assert_eq!(lines, vec!["abc"]);
    }

    #[test]
    fn serialize_with_color() {
        let mut c = Canvas::new(3, 1);
        let red = Some(CanvasColor::new(255, 0, 0));
        c.set(0, 0, 'a', red);
        c.set(1, 0, 'b', red);
        c.set(2, 0, 'c', None);
        let lines = c.to_lines();
        assert_eq!(lines, vec!["{#FF0000}ab{/}c"]);
    }

    #[test]
    fn execute_all_full_script() {
        let script = "\
CLEAR
FILL . red
RECT 0 0 3 2 # blue
TEXT 5 0 hi green
HLINE 3 0 9 -
";
        let cmds = parse_script(script);
        let mut c = Canvas::new(10, 5);
        c.execute_all(&cmds);

        // RECT overwrote the fill in top-left
        assert_eq!(c.get(0, 0).unwrap().ch, '#');
        assert_eq!(
            c.get(0, 0).unwrap().color,
            Some(CanvasColor::new(0, 0, 238))
        );

        // TEXT wrote "hi" at (5,0)
        assert_eq!(c.get(5, 0).unwrap().ch, 'h');
        assert_eq!(c.get(6, 0).unwrap().ch, 'i');

        // HLINE at y=3
        assert_eq!(c.get(0, 3).unwrap().ch, '-');
        assert_eq!(c.get(9, 3).unwrap().ch, '-');

        // Fill is still present where not overwritten
        assert_eq!(c.get(5, 4).unwrap().ch, '.');
        assert_eq!(
            c.get(5, 4).unwrap().color,
            Some(CanvasColor::new(205, 0, 0))
        );
    }
}
