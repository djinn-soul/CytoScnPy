//! Byte offsets and line numbers to LSP positions (UTF-16 code units).

use lsp_types::{Position, Range};

/// Line starts of a text, for converting offsets to positions.
pub(crate) struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { text, starts }
    }

    /// The position of byte `offset`, clamped to the text and moved back to
    /// a character boundary.
    pub(crate) fn position(&self, offset: usize) -> Position {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.starts.partition_point(|&start| start <= offset) - 1;
        let start = self.starts[line];
        Position::new(to_u32(line), utf16_len(&self.text[start..offset]))
    }

    /// The text of line `line` (0-indexed) without its line break.
    fn line_text(&self, line: usize) -> Option<&'a str> {
        let start = *self.starts.get(line)?;
        let end = self
            .starts
            .get(line + 1)
            .map_or(self.text.len(), |&next| next - 1);
        let text = &self.text[start..end];
        Some(text.strip_suffix('\r').unwrap_or(text))
    }

    /// The range of line `line` (1-indexed) from its first non-blank
    /// character to its end, or the start of the file for a line that does
    /// not exist.
    pub(crate) fn line_range(&self, line: usize) -> Range {
        let Some(text) = line.checked_sub(1).and_then(|index| self.line_text(index)) else {
            return Range::default();
        };
        let indent = text.len() - text.trim_start().len();
        let line = to_u32(line - 1);
        Range::new(
            Position::new(line, utf16_len(&text[..indent])),
            Position::new(line, utf16_len(text)),
        )
    }

    /// The range of bytes `start..end`.
    pub(crate) fn range(&self, start: usize, end: usize) -> Range {
        Range::new(self.position(start), self.position(end))
    }

    /// Whether `text[start..]` begins with `name`.
    pub(crate) fn has_text_at(&self, start: usize, name: &str) -> bool {
        self.text
            .get(start..)
            .is_some_and(|rest| rest.starts_with(name))
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }
}

fn utf16_len(text: &str) -> u32 {
    to_u32(text.encode_utf16().count())
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::LineIndex;
    use lsp_types::{Position, Range};

    #[test]
    fn counts_columns_in_utf16_code_units() {
        let index = LineIndex::new("é = 1\n😀x = 2\n");
        assert_eq!(index.position(0), Position::new(0, 0));
        assert_eq!(index.position(2), Position::new(0, 1));
        // `é` is 2 bytes, so line 1 starts at byte 7; `😀` is 4 bytes but
        // 2 UTF-16 code units.
        assert_eq!(index.position(7), Position::new(1, 0));
        assert_eq!(index.position(11), Position::new(1, 2));
        assert_eq!(index.position(12), Position::new(1, 3));
        // Inside the emoji, moved back to its start; past the end, clamped.
        assert_eq!(index.position(9), Position::new(1, 0));
        assert_eq!(index.position(99), Position::new(2, 0));
    }

    #[test]
    fn line_ranges_skip_indentation_and_line_breaks() {
        let index = LineIndex::new("def f():\r\n    eval(x)\r\n");
        assert_eq!(
            index.line_range(2),
            Range::new(Position::new(1, 4), Position::new(1, 11))
        );
        assert_eq!(index.line_range(0), Range::default());
        assert_eq!(index.line_range(9), Range::default());
    }
}
