//! Bitmap font for BIGTEXT rendering in the canvas language.
//!
//! Each glyph is a 3x5 bitmap (5 rows, 3 columns).

const T: bool = true;
const F: bool = false;

/// 40 glyphs: A-Z (0-25), 0-9 (26-35), space (36), ! (37), ? (38), . (39)
const FONT: [[[bool; 3]; 5]; 40] = [
    // A
    [
        [F, T, F],
        [T, F, T],
        [T, T, T],
        [T, F, T],
        [T, F, T],
    ],
    // B
    [
        [T, T, F],
        [T, F, T],
        [T, T, F],
        [T, F, T],
        [T, T, F],
    ],
    // C
    [
        [F, T, T],
        [T, F, F],
        [T, F, F],
        [T, F, F],
        [F, T, T],
    ],
    // D
    [
        [T, T, F],
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [T, T, F],
    ],
    // E
    [
        [T, T, T],
        [T, F, F],
        [T, T, F],
        [T, F, F],
        [T, T, T],
    ],
    // F
    [
        [T, T, T],
        [T, F, F],
        [T, T, F],
        [T, F, F],
        [T, F, F],
    ],
    // G
    [
        [F, T, T],
        [T, F, F],
        [T, F, T],
        [T, F, T],
        [F, T, T],
    ],
    // H
    [
        [T, F, T],
        [T, F, T],
        [T, T, T],
        [T, F, T],
        [T, F, T],
    ],
    // I
    [
        [T, T, T],
        [F, T, F],
        [F, T, F],
        [F, T, F],
        [T, T, T],
    ],
    // J
    [
        [F, F, T],
        [F, F, T],
        [F, F, T],
        [T, F, T],
        [F, T, F],
    ],
    // K
    [
        [T, F, T],
        [T, F, T],
        [T, T, F],
        [T, F, T],
        [T, F, T],
    ],
    // L
    [
        [T, F, F],
        [T, F, F],
        [T, F, F],
        [T, F, F],
        [T, T, T],
    ],
    // M
    [
        [T, F, T],
        [T, T, T],
        [T, T, T],
        [T, F, T],
        [T, F, T],
    ],
    // N
    [
        [T, F, T],
        [T, T, T],
        [T, T, T],
        [T, T, T],
        [T, F, T],
    ],
    // O
    [
        [F, T, F],
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [F, T, F],
    ],
    // P
    [
        [T, T, F],
        [T, F, T],
        [T, T, F],
        [T, F, F],
        [T, F, F],
    ],
    // Q
    [
        [F, T, F],
        [T, F, T],
        [T, F, T],
        [T, T, F],
        [F, T, T],
    ],
    // R
    [
        [T, T, F],
        [T, F, T],
        [T, T, F],
        [T, F, T],
        [T, F, T],
    ],
    // S
    [
        [F, T, T],
        [T, F, F],
        [F, T, F],
        [F, F, T],
        [T, T, F],
    ],
    // T
    [
        [T, T, T],
        [F, T, F],
        [F, T, F],
        [F, T, F],
        [F, T, F],
    ],
    // U
    [
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [F, T, F],
    ],
    // V
    [
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [F, T, F],
    ],
    // W
    [
        [T, F, T],
        [T, F, T],
        [T, T, T],
        [T, T, T],
        [T, F, T],
    ],
    // X
    [
        [T, F, T],
        [T, F, T],
        [F, T, F],
        [T, F, T],
        [T, F, T],
    ],
    // Y
    [
        [T, F, T],
        [T, F, T],
        [F, T, F],
        [F, T, F],
        [F, T, F],
    ],
    // Z
    [
        [T, T, T],
        [F, F, T],
        [F, T, F],
        [T, F, F],
        [T, T, T],
    ],
    // 0
    [
        [F, T, F],
        [T, F, T],
        [T, F, T],
        [T, F, T],
        [F, T, F],
    ],
    // 1
    [
        [F, T, F],
        [T, T, F],
        [F, T, F],
        [F, T, F],
        [T, T, T],
    ],
    // 2
    [
        [F, T, F],
        [T, F, T],
        [F, F, T],
        [F, T, F],
        [T, T, T],
    ],
    // 3
    [
        [T, T, F],
        [F, F, T],
        [F, T, F],
        [F, F, T],
        [T, T, F],
    ],
    // 4
    [
        [T, F, T],
        [T, F, T],
        [T, T, T],
        [F, F, T],
        [F, F, T],
    ],
    // 5
    [
        [T, T, T],
        [T, F, F],
        [T, T, F],
        [F, F, T],
        [T, T, F],
    ],
    // 6
    [
        [F, T, T],
        [T, F, F],
        [T, T, F],
        [T, F, T],
        [F, T, F],
    ],
    // 7
    [
        [T, T, T],
        [F, F, T],
        [F, T, F],
        [F, T, F],
        [F, T, F],
    ],
    // 8
    [
        [F, T, F],
        [T, F, T],
        [F, T, F],
        [T, F, T],
        [F, T, F],
    ],
    // 9
    [
        [F, T, F],
        [T, F, T],
        [F, T, T],
        [F, F, T],
        [T, T, F],
    ],
    // space (36)
    [
        [F, F, F],
        [F, F, F],
        [F, F, F],
        [F, F, F],
        [F, F, F],
    ],
    // ! (37)
    [
        [F, T, F],
        [F, T, F],
        [F, T, F],
        [F, F, F],
        [F, T, F],
    ],
    // ? (38)
    [
        [F, T, F],
        [T, F, T],
        [F, F, T],
        [F, T, F],
        [F, T, F],
    ],
    // . (39)
    [
        [F, F, F],
        [F, F, F],
        [F, F, F],
        [F, F, F],
        [F, T, F],
    ],
];

/// Returns the 3x5 bitmap glyph for the given character, or `None` if not found.
///
/// Lowercase letters are mapped to uppercase. Supported: A-Z, 0-9, space, !, ?, .
pub fn get_glyph(ch: char) -> Option<&'static [[bool; 3]; 5]> {
    let idx = match ch.to_ascii_uppercase() {
        c @ 'A'..='Z' => (c as u8 - b'A') as usize,
        c @ '0'..='9' => 26 + (c as u8 - b'0') as usize,
        ' ' => 36,
        '!' => 37,
        '?' => 38,
        '.' => 39,
        _ => return None,
    };
    Some(&FONT[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_exists() {
        assert!(get_glyph('A').is_some());
        assert!(get_glyph('z').is_some());
        assert!(get_glyph('0').is_some());
        assert!(get_glyph('!').is_some());
    }

    #[test]
    fn glyph_not_found() {
        assert!(get_glyph('@').is_none());
        assert!(get_glyph('*').is_none());
    }

    #[test]
    fn glyph_dimensions() {
        let g = get_glyph('A').unwrap();
        assert_eq!(g.len(), 5);
        for row in g {
            assert_eq!(row.len(), 3);
        }
    }

    #[test]
    fn space_is_blank() {
        let g = get_glyph(' ').unwrap();
        for row in g {
            for &pixel in row {
                assert!(!pixel);
            }
        }
    }

    #[test]
    fn lowercase_maps_to_uppercase() {
        assert_eq!(get_glyph('a'), get_glyph('A'));
        assert_eq!(get_glyph('m'), get_glyph('M'));
    }
}
