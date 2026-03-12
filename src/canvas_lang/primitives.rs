//! Drawing primitives for the canvas language (lines, rectangles, circles, etc.).

use crate::canvas_lang::color::CanvasColor;
use crate::canvas_lang::font;
use crate::canvas_lang::parser::{GradientDir, PatternType};
use crate::canvas_lang::renderer::Canvas;

/// Draws a filled rectangle.
pub fn filled_rect(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    ch: char,
    color: Option<CanvasColor>,
) {
    for dy in 0..h as i32 {
        for dx in 0..w as i32 {
            canvas.set(x + dx, y + dy, ch, color);
        }
    }
}

/// Draws a rectangle outline (edges only).
pub fn outline_rect(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let w = w as i32;
    let h = h as i32;
    for dx in 0..w {
        canvas.set(x + dx, y, ch, color);
        canvas.set(x + dx, y + h - 1, ch, color);
    }
    for dy in 0..h {
        canvas.set(x, y + dy, ch, color);
        canvas.set(x + w - 1, y + dy, ch, color);
    }
}

/// Draws a box frame using box-drawing characters: ┌┐└┘─│
pub fn frame_box(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Option<CanvasColor>,
) {
    let w = w as i32;
    let h = h as i32;
    if w < 2 || h < 2 {
        return;
    }
    canvas.set(x, y, '\u{250C}', color); // ┌
    canvas.set(x + w - 1, y, '\u{2510}', color); // ┐
    canvas.set(x, y + h - 1, '\u{2514}', color); // └
    canvas.set(x + w - 1, y + h - 1, '\u{2518}', color); // ┘
    for dx in 1..w - 1 {
        canvas.set(x + dx, y, '\u{2500}', color); // ─
        canvas.set(x + dx, y + h - 1, '\u{2500}', color);
    }
    for dy in 1..h - 1 {
        canvas.set(x, y + dy, '\u{2502}', color); // │
        canvas.set(x + w - 1, y + dy, '\u{2502}', color);
    }
}

/// Draws a rounded box using rounded box-drawing characters: ╭╮╰╯─│
pub fn round_box(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Option<CanvasColor>,
) {
    let w = w as i32;
    let h = h as i32;
    if w < 2 || h < 2 {
        return;
    }
    canvas.set(x, y, '\u{256D}', color); // ╭
    canvas.set(x + w - 1, y, '\u{256E}', color); // ╮
    canvas.set(x, y + h - 1, '\u{2570}', color); // ╰
    canvas.set(x + w - 1, y + h - 1, '\u{256F}', color); // ╯
    for dx in 1..w - 1 {
        canvas.set(x + dx, y, '\u{2500}', color); // ─
        canvas.set(x + dx, y + h - 1, '\u{2500}', color);
    }
    for dy in 1..h - 1 {
        canvas.set(x, y + dy, '\u{2502}', color); // │
        canvas.set(x + w - 1, y + dy, '\u{2502}', color);
    }
}

/// Draws a line using Bresenham's algorithm.
pub fn bresenham_line(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let mut x = x1;
    let mut y = y1;
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        canvas.set(x, y, ch, color);
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draws a line with an arrowhead at the end.
pub fn arrow(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Option<CanvasColor>,
) {
    // Draw the line body with a generic char
    bresenham_line(canvas, x1, y1, x2, y2, '-', color);
    // Replace the endpoint with an arrowhead
    let dx = x2 - x1;
    let dy = y2 - y1;
    let head = if dx.abs() >= dy.abs() {
        if dx >= 0 { '\u{2192}' } else { '\u{2190}' } // → ←
    } else if dy >= 0 {
        '\u{2193}' // ↓
    } else {
        '\u{2191}' // ↑
    };
    canvas.set(x2, y2, head, color);
}

/// Draws an L-shaped or straight line using box-drawing characters.
/// For L-shape: horizontal first to x2, then vertical to y2.
pub fn box_line(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Option<CanvasColor>,
) {
    if y1 == y2 {
        // Horizontal line
        let (sx, ex) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        for x in sx..=ex {
            canvas.set(x, y1, '\u{2500}', color); // ─
        }
    } else if x1 == x2 {
        // Vertical line
        let (sy, ey) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in sy..=ey {
            canvas.set(x1, y, '\u{2502}', color); // │
        }
    } else {
        // L-shaped: horizontal from x1 to x2 at y1, then vertical from y1 to y2 at x2
        let (sx, ex) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        for x in sx..=ex {
            canvas.set(x, y1, '\u{2500}', color); // ─
        }
        let (sy, ey) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in sy..=ey {
            canvas.set(x2, y, '\u{2502}', color); // │
        }
        // Corner character at the bend
        let corner = if y2 > y1 {
            '\u{2510}' // ┐
        } else {
            '\u{2518}' // ┘
        };
        canvas.set(x2, y1, corner, color);
    }
}

/// Draws a filled circle with 2:1 aspect ratio compensation.
pub fn filled_circle(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    r: u32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let r = r as i32;
    for dy in -r..=r {
        for dx in -r * 2..=r * 2 {
            if dx * dx + dy * dy * 4 <= r * r * 4 {
                canvas.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

/// Draws a ring (circle border only).
pub fn ring(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    r: u32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let r = r as i32;
    let r_inner = r - 1;
    for dy in -r..=r {
        for dx in -r * 2..=r * 2 {
            let dist = dx * dx + dy * dy * 4;
            if dist <= r * r * 4 && dist > r_inner * r_inner * 4 {
                canvas.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

/// Draws a filled ellipse.
pub fn filled_ellipse(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    rx: u32,
    ry: u32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let rx = rx as i32;
    let ry = ry as i32;
    if rx == 0 || ry == 0 {
        return;
    }
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            // Standard ellipse equation: (dx/rx)^2 + (dy/ry)^2 <= 1
            // Multiply through: dx^2 * ry^2 + dy^2 * rx^2 <= rx^2 * ry^2
            if (dx * dx) as i64 * (ry * ry) as i64 + (dy * dy) as i64 * (rx * rx) as i64
                <= (rx * rx) as i64 * (ry * ry) as i64
            {
                canvas.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

/// Draws a filled triangle using bounding box + point-in-triangle test.
#[allow(clippy::too_many_arguments)]
pub fn filled_triangle(
    canvas: &mut Canvas,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    x3: i32,
    y3: i32,
    ch: char,
    color: Option<CanvasColor>,
) {
    let min_x = x1.min(x2).min(x3);
    let max_x = x1.max(x2).max(x3);
    let min_y = y1.min(y2).min(y3);
    let max_y = y1.max(y2).max(y3);

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            if point_in_triangle(px, py, x1, y1, x2, y2, x3, y3) {
                canvas.set(px, py, ch, color);
            }
        }
    }
}

fn sign(x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) -> i64 {
    (x1 as i64 - x3 as i64) * (y2 as i64 - y3 as i64)
        - (x2 as i64 - x3 as i64) * (y1 as i64 - y3 as i64)
}

#[allow(clippy::too_many_arguments)]
fn point_in_triangle(
    px: i32,
    py: i32,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    x3: i32,
    y3: i32,
) -> bool {
    let d1 = sign(px, py, x1, y1, x2, y2);
    let d2 = sign(px, py, x2, y2, x3, y3);
    let d3 = sign(px, py, x3, y3, x1, y1);

    let has_neg = (d1 < 0) || (d2 < 0) || (d3 < 0);
    let has_pos = (d1 > 0) || (d2 > 0) || (d3 > 0);

    !(has_neg && has_pos)
}

/// Fills an area with gradient characters based on direction.
pub fn gradient(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    direction: GradientDir,
) {
    let chars = ['\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}']; // ░▒▓█
    let w_i = w as i32;
    let h_i = h as i32;
    for dy in 0..h_i {
        for dx in 0..w_i {
            let t = match direction {
                GradientDir::Right => {
                    if w_i <= 1 {
                        1.0
                    } else {
                        dx as f64 / (w_i - 1) as f64
                    }
                }
                GradientDir::Left => {
                    if w_i <= 1 {
                        1.0
                    } else {
                        1.0 - dx as f64 / (w_i - 1) as f64
                    }
                }
                GradientDir::Down => {
                    if h_i <= 1 {
                        1.0
                    } else {
                        dy as f64 / (h_i - 1) as f64
                    }
                }
                GradientDir::Up => {
                    if h_i <= 1 {
                        1.0
                    } else {
                        1.0 - dy as f64 / (h_i - 1) as f64
                    }
                }
            };
            let idx = (t * 3.999).min(3.0) as usize;
            canvas.set(x + dx, y + dy, chars[idx], None);
        }
    }
}

/// Fills an area with a repeating pattern.
pub fn pattern(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    pattern_type: PatternType,
    color: Option<CanvasColor>,
) {
    let w_i = w as i32;
    let h_i = h as i32;
    for dy in 0..h_i {
        for dx in 0..w_i {
            let ch = match pattern_type {
                PatternType::Checker => {
                    if (dx + dy) % 2 == 0 {
                        '\u{2588}' // █
                    } else {
                        ' '
                    }
                }
                PatternType::Dots => {
                    if dx % 2 == 0 && dy % 2 == 0 {
                        '\u{00B7}' // ·
                    } else {
                        ' '
                    }
                }
                PatternType::StripesH => {
                    if dy % 2 == 0 {
                        '\u{2500}' // ─
                    } else {
                        ' '
                    }
                }
                PatternType::StripesV => {
                    if dx % 2 == 0 {
                        '\u{2502}' // │
                    } else {
                        ' '
                    }
                }
                PatternType::Cross => {
                    let h_stripe = dy % 2 == 0;
                    let v_stripe = dx % 2 == 0;
                    if h_stripe && v_stripe {
                        '+'
                    } else if h_stripe {
                        '\u{2500}' // ─
                    } else if v_stripe {
                        '\u{2502}' // │
                    } else {
                        ' '
                    }
                }
            };
            canvas.set(x + dx, y + dy, ch, color);
        }
    }
}

/// Renders text using the bitmap font (3x5 glyphs).
pub fn big_text(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    text: &str,
    color: Option<CanvasColor>,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if let Some(glyph) = font::get_glyph(ch) {
            for (row_idx, row) in glyph.iter().enumerate() {
                for (col_idx, &pixel) in row.iter().enumerate() {
                    if pixel {
                        canvas.set(
                            cursor_x + col_idx as i32,
                            y + row_idx as i32,
                            '\u{2588}', // █
                            color,
                        );
                    }
                }
            }
            cursor_x += 3 + 1; // glyph width + 1 spacing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_canvas(w: usize, h: usize) -> Canvas {
        Canvas::new(w, h)
    }

    #[test]
    fn test_filled_rect() {
        let mut c = make_canvas(10, 10);
        filled_rect(&mut c, 1, 1, 3, 2, '#', None);
        assert_eq!(c.get(1, 1).unwrap().ch, '#');
        assert_eq!(c.get(3, 2).unwrap().ch, '#');
        assert_eq!(c.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_outline_rect() {
        let mut c = make_canvas(10, 10);
        outline_rect(&mut c, 0, 0, 5, 3, '#', None);
        assert_eq!(c.get(0, 0).unwrap().ch, '#');
        assert_eq!(c.get(2, 0).unwrap().ch, '#');
        assert_eq!(c.get(2, 1).unwrap().ch, ' '); // interior
    }

    #[test]
    fn test_frame_box_corners() {
        let mut c = make_canvas(10, 10);
        frame_box(&mut c, 0, 0, 5, 3, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '┌');
        assert_eq!(c.get(4, 0).unwrap().ch, '┐');
        assert_eq!(c.get(0, 2).unwrap().ch, '└');
        assert_eq!(c.get(4, 2).unwrap().ch, '┘');
    }

    #[test]
    fn test_round_box_corners() {
        let mut c = make_canvas(10, 10);
        round_box(&mut c, 0, 0, 5, 3, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '╭');
        assert_eq!(c.get(4, 0).unwrap().ch, '╮');
        assert_eq!(c.get(0, 2).unwrap().ch, '╰');
        assert_eq!(c.get(4, 2).unwrap().ch, '╯');
    }

    #[test]
    fn test_bresenham_horizontal() {
        let mut c = make_canvas(10, 10);
        bresenham_line(&mut c, 0, 0, 4, 0, '-', None);
        for x in 0..=4 {
            assert_eq!(c.get(x, 0).unwrap().ch, '-');
        }
    }

    #[test]
    fn test_bresenham_diagonal() {
        let mut c = make_canvas(10, 10);
        bresenham_line(&mut c, 0, 0, 3, 3, '*', None);
        for i in 0..=3 {
            assert_eq!(c.get(i, i).unwrap().ch, '*');
        }
    }

    #[test]
    fn test_filled_circle_center_and_far() {
        let mut c = make_canvas(40, 20);
        filled_circle(&mut c, 20, 10, 5, '*', None);
        assert_eq!(c.get(20, 10).unwrap().ch, '*'); // center
        assert_eq!(c.get(0, 0).unwrap().ch, ' '); // far away
    }

    #[test]
    fn test_gradient_right_endpoints() {
        let mut c = make_canvas(10, 1);
        gradient(&mut c, 0, 0, 10, 1, GradientDir::Right);
        assert_eq!(c.get(0, 0).unwrap().ch, '░');
        assert_eq!(c.get(9, 0).unwrap().ch, '█');
    }

    #[test]
    fn test_pattern_checker() {
        let mut c = make_canvas(4, 4);
        pattern(&mut c, 0, 0, 4, 4, PatternType::Checker, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '█');
        assert_eq!(c.get(1, 0).unwrap().ch, ' ');
        assert_eq!(c.get(1, 1).unwrap().ch, '█');
    }

    #[test]
    fn test_filled_triangle_vertex_and_interior() {
        let mut c = make_canvas(20, 20);
        filled_triangle(&mut c, 5, 0, 0, 10, 10, 10, '^', None);
        // Vertices should be filled
        assert_eq!(c.get(5, 0).unwrap().ch, '^');
        assert_eq!(c.get(0, 10).unwrap().ch, '^');
        assert_eq!(c.get(10, 10).unwrap().ch, '^');
        // Interior point
        assert_eq!(c.get(5, 5).unwrap().ch, '^');
    }

    #[test]
    fn test_arrow_head_char() {
        let mut c = make_canvas(20, 1);
        arrow(&mut c, 0, 0, 10, 0, None);
        assert_eq!(c.get(10, 0).unwrap().ch, '→');
    }

    #[test]
    fn test_clipping_negative_coords() {
        let mut c = make_canvas(10, 10);
        // Should not panic even with negative start coordinates
        filled_rect(&mut c, -5, -5, 10, 10, '#', None);
        assert_eq!(c.get(0, 0).unwrap().ch, '#');
        assert_eq!(c.get(4, 4).unwrap().ch, '#');
        assert_eq!(c.get(5, 5).unwrap().ch, ' ');
    }
}
