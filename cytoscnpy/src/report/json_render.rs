//! Streaming, parallel pretty-printing of the machine JSON payload.
//!
//! Large scans produce hundreds of megabytes of JSON. Rendering array
//! elements on worker threads and streaming batches to the writer avoids
//! materializing the whole document as a `serde_json::Value` tree and string.
//! The bytes match `serde_json::to_string_pretty(&serde_json::to_value(..))`:
//! keys sorted at every level, two-space indentation.

use std::io::Write;

use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;

/// Indentation used by `serde_json::ser::PrettyFormatter`.
const INDENT: &[u8] = b"  ";
/// Indentation of an element inside a top-level array.
const ELEMENT_INDENT: &[u8] = b"    ";
/// Elements rendered per parallel batch; bounds buffered output memory.
const BATCH: usize = 16_384;
/// Elements one task renders into a single buffer.
const CHUNK: usize = 128;

/// Writes one top-level value of the payload.
pub(super) type Section<'a> = Box<dyn Fn(&mut dyn Write) -> Result<()> + 'a>;

/// Writes `sections` as a pretty JSON object with keys in sorted order.
pub(super) fn write_object(
    out: &mut dyn Write,
    mut sections: Vec<(&str, Section<'_>)>,
) -> Result<()> {
    // Matches the key order of `serde_json::Map` without `preserve_order`.
    sections.sort_by(|a, b| a.0.cmp(b.0));
    if sections.is_empty() {
        out.write_all(b"{}")?;
        return Ok(());
    }
    out.write_all(b"{\n")?;
    for (index, (key, section)) in sections.iter().enumerate() {
        if index > 0 {
            out.write_all(b",\n")?;
        }
        out.write_all(INDENT)?;
        serde_json::to_writer(&mut *out, key)?;
        out.write_all(b": ")?;
        section(out)?;
    }
    out.write_all(b"\n}")?;
    Ok(())
}

/// A section holding a serializable array. The intermediate `Value` sorts
/// each element's keys.
pub(super) fn array<T: Serialize + Sync>(items: &[T]) -> Section<'_> {
    Box::new(move |out| {
        write_array(out, items, |item, buffer| {
            serde_json::to_writer_pretty(buffer, &serde_json::to_value(item)?)
        })
    })
}

/// A section holding an array whose items already serialize their keys in
/// sorted order, such as JSON values.
pub(super) fn sorted_keys<T: Serialize + Sync>(items: &[T]) -> Section<'_> {
    Box::new(move |out| {
        write_array(out, items, |item, buffer| {
            serde_json::to_writer_pretty(buffer, item)
        })
    })
}

/// A section serializing each item through `view`, a borrowed form whose
/// keys are already in sorted order.
pub(super) fn sorted_view<'a, T, V>(items: &'a [T], view: fn(&'a T) -> V) -> Section<'a>
where
    T: Sync,
    V: Serialize + 'a,
{
    Box::new(move |out| {
        write_array(out, items, |item, buffer| {
            serde_json::to_writer_pretty(buffer, &view(item))
        })
    })
}

/// A section holding a single serializable value.
pub(super) fn value<T: Serialize>(item: &T) -> Section<'_> {
    Box::new(move |out| {
        let text = serde_json::to_vec_pretty(&serde_json::to_value(item)?)?;
        let mut indented = Vec::with_capacity(text.len());
        push_indented(&mut indented, &text, INDENT);
        out.write_all(&indented)?;
        Ok(())
    })
}

fn write_array<'a, T, F>(out: &mut dyn Write, items: &'a [T], render: F) -> Result<()>
where
    T: Sync,
    F: Fn(&'a T, &mut Vec<u8>) -> serde_json::Result<()> + Sync,
{
    if items.is_empty() {
        out.write_all(b"[]")?;
        return Ok(());
    }
    out.write_all(b"[\n")?;
    for (batch_index, batch) in items.chunks(BATCH).enumerate() {
        // One buffer per chunk keeps allocations, and the frees on this
        // thread afterwards, per chunk rather than per element.
        let chunks = batch
            .par_chunks(CHUNK)
            .enumerate()
            .map(|(chunk_index, chunk)| {
                let mut rendered = Vec::new();
                let mut element = Vec::new();
                for (index, item) in chunk.iter().enumerate() {
                    if batch_index > 0 || chunk_index > 0 || index > 0 {
                        rendered.extend_from_slice(b",\n");
                    }
                    rendered.extend_from_slice(ELEMENT_INDENT);
                    element.clear();
                    render(item, &mut element)?;
                    push_indented(&mut rendered, &element, ELEMENT_INDENT);
                }
                Ok(rendered)
            })
            .collect::<serde_json::Result<Vec<Vec<u8>>>>()?;
        for chunk in chunks {
            out.write_all(&chunk)?;
        }
    }
    out.write_all(b"\n")?;
    out.write_all(INDENT)?;
    out.write_all(b"]")?;
    Ok(())
}

/// Appends `text`, shifting every continuation line right by `indent`. Pretty
/// JSON never holds a raw newline inside a string, so each newline starts a
/// structural line.
fn push_indented(out: &mut Vec<u8>, text: &[u8], indent: &[u8]) {
    let mut lines = text.split(|byte| *byte == b'\n');
    if let Some(first) = lines.next() {
        out.extend_from_slice(first);
    }
    for line in lines {
        out.push(b'\n');
        out.extend_from_slice(indent);
        out.extend_from_slice(line);
    }
}
