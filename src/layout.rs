use std::fmt::{Display, Write as _};
use std::iter;

/// Marker denoting a blank cell.
const B: u8 = 0x80;

/// Physical layout of US keymap.
#[rustfmt::skip]
pub const US_LAYOUT_WIDTHS_MAP: [[u8; 15]; 8] = [
    [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5], // Esc, 0, .., ~
    [7, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 8, 8], // Tab, Q, .., Delete (/BS)
    [9, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, B, B, 11], // Control, A, .., Return
    [11, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, B, B, 9, 5], // Shift, Z, .., Fn
    [7 | B, B, 5, 8, B, 29, B, B, 8, 5, B, B, B, B, B], // Alt, ..., Alt
    [23 | B, B, B, B, 8, 5, 8, B, B, B, 31 | B, 5, 5, B, B], // Left, Middle, Right, gesture pad?
    [75 | B, B, B, B, B, B, B, B, B, B, B, 5, 5, B, B], // gesture pad?
    [75 | B, B, B, 5, 5, B, B, B, B, B, B, 5, 5, B, B], // gesture pad?
];

/// Formats the row `labels` based on the given `widths` layout table.
pub fn format_row<I>(widths: &[u8], labels: I) -> String
where
    I: IntoIterator,
    I::Item: Display,
{
    let mut line = String::new();
    let mut was_blank = true;
    for (label, &width) in labels.into_iter().zip(widths) {
        let is_blank = width & B != 0;
        let mut width = usize::from(width & !B);
        if width == 0 {
            continue;
        }
        if is_blank && was_blank {
            line.push(' ');
        } else {
            line.push('|');
        }
        width -= 1;
        if is_blank {
            line.extend(iter::repeat(' ').take(width));
        } else {
            let max_len = line.len() + width;
            write!(&mut line, "{label:width$}").unwrap();
            line.truncate(max_len);
        }
        was_blank = is_blank;
    }
    if !was_blank {
        line.push('|');
    }
    line
}
