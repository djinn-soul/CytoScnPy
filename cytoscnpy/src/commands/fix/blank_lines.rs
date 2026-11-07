//! Remove the lines that whole-line deletions empty, and keep the blank-line
//! spacing around them as it was.
//!
//! Deleting a definition by its syntax range leaves its (now empty) line and
//! the blank lines on both sides of it, so removing a function between two
//! others left five blank lines where two separated them before. A deletion
//! that spans whole lines takes those lines, and the blank lines around it
//! shrink to the larger of the two runs. At the start or end of the file
//! there is nothing to separate, so the blank lines there go too.

use super::apply_plan::PlannedEdit;

/// Widens whole-line deletions in `planned`, which is sorted by start and
/// holds no overlapping edits.
pub(super) fn absorb_blank_lines(content: &str, planned: &mut [PlannedEdit]) {
    let bytes = content.as_bytes();
    let whole: Vec<bool> = planned
        .iter_mut()
        .map(|edit| {
            if edit.replacement.is_some() {
                return false;
            }
            let Some((start, end)) = whole_lines(bytes, edit.start_byte, edit.end_byte) else {
                return false;
            };
            edit.start_byte = start;
            edit.end_byte = end;
            true
        })
        .collect();

    // Start of the file region already emptied by earlier deletions.
    let mut cleared_to = 0;
    // End of the last deletion, which blank lines are never taken across.
    let mut floor = 0;
    let mut index = 0;
    while index < planned.len() {
        if !whole[index] {
            if planned[index].replacement.is_none() {
                floor = planned[index].end_byte;
            }
            index += 1;
            continue;
        }
        // Adjacent whole-line deletions empty one run of lines together.
        let first = index;
        while index + 1 < planned.len()
            && whole[index + 1]
            && planned[index + 1].start_byte == planned[index].end_byte
        {
            index += 1;
        }
        let last = index;
        index += 1;

        let (before_start, before) = blank_lines_before(bytes, planned[first].start_byte, floor);
        let (after_end, after) = blank_lines_after(bytes, planned[last].end_byte, usize::MAX);
        if before_start == cleared_to {
            planned[first].start_byte = before_start;
            planned[last].end_byte = after_end;
            cleared_to = after_end;
        } else if after_end == bytes.len() {
            planned[first].start_byte = before_start;
            planned[last].end_byte = after_end;
        } else {
            let keep_end = planned[last].end_byte;
            planned[last].end_byte = blank_lines_after(bytes, keep_end, before.min(after)).0;
        }
        floor = planned[last].end_byte;
    }
}

/// The full lines `start..end` covers, when only spaces and tabs share its
/// first and last lines.
fn whole_lines(bytes: &[u8], start: usize, end: usize) -> Option<(usize, usize)> {
    if start >= end {
        return None;
    }
    let line_start = bytes[..start]
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |newline| newline + 1);
    if !is_blank(&bytes[line_start..start]) {
        return None;
    }
    let line_end = bytes[end..]
        .iter()
        .position(|&byte| byte == b'\n')
        .map_or(bytes.len(), |newline| end + newline + 1);
    is_blank(&bytes[end..line_end]).then_some((line_start, line_end))
}

/// Where the blank lines just before the line at `start` begin, and how many
/// there are, looking no further back than `floor`.
fn blank_lines_before(bytes: &[u8], start: usize, floor: usize) -> (usize, usize) {
    let mut position = start;
    let mut count = 0;
    while position > floor {
        let line_start = bytes[..position - 1]
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(0, |newline| newline + 1);
        if line_start < floor || !is_blank(&bytes[line_start..position]) {
            break;
        }
        position = line_start;
        count += 1;
    }
    (position, count)
}

/// Where at most `limit` blank lines from the line start `end` stop, and how
/// many there are.
fn blank_lines_after(bytes: &[u8], end: usize, limit: usize) -> (usize, usize) {
    let mut position = end;
    let mut count = 0;
    while position < bytes.len() && count < limit {
        let line_end = bytes[position..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(bytes.len(), |newline| position + newline + 1);
        if !is_blank(&bytes[position..line_end]) {
            break;
        }
        position = line_end;
        count += 1;
    }
    (position, count)
}

fn is_blank(text: &[u8]) -> bool {
    text.iter()
        .all(|&byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c'))
}
