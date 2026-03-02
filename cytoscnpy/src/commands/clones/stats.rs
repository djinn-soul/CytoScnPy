use crate::clones::{ClonePair, CloneType};
use anyhow::Result;
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

pub(super) fn load_matched_files(pairs: &[ClonePair]) -> Vec<(PathBuf, String)> {
    let unique_paths: HashSet<PathBuf> = pairs
        .iter()
        .flat_map(|p| [p.instance_a.file.clone(), p.instance_b.file.clone()])
        .collect();

    unique_paths
        .into_iter()
        .filter_map(|path| {
            std::fs::read_to_string(&path)
                .ok()
                .map(|content| (path, content))
        })
        .collect()
}

pub(super) fn print_clone_stats_simple<W: Write>(
    mut writer: W,
    file_count: usize,
    pairs: &[ClonePair],
) -> Result<()> {
    writeln!(writer, "[VERBOSE] Clone Detection Statistics:")?;
    writeln!(writer, "   Files scanned: {file_count}")?;
    writeln!(writer, "   Clone pairs found: {}", pairs.len())?;

    let mut type1_count = 0;
    let mut type2_count = 0;
    let mut type3_count = 0;
    for pair in pairs {
        match pair.clone_type {
            CloneType::Type1 => type1_count += 1,
            CloneType::Type2 => type2_count += 1,
            CloneType::Type3 => type3_count += 1,
        }
    }
    writeln!(writer, "   Exact Copies: {type1_count}")?;
    writeln!(writer, "   Renamed Copies: {type2_count}")?;
    writeln!(writer, "   Similar Code: {type3_count}")?;

    if !pairs.is_empty() {
        #[allow(clippy::cast_precision_loss)]
        let avg_similarity: f64 =
            pairs.iter().map(|p| p.similarity).sum::<f64>() / pairs.len() as f64;
        writeln!(
            writer,
            "   Average similarity: {:.0}%",
            avg_similarity * 100.0
        )?;
    }
    writeln!(writer)?;
    Ok(())
}
