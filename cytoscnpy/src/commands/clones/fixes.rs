use crate::clones::CloneFinding;
use crate::fix::{ByteRangeRewriter, Edit};
use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub(super) fn apply_clone_fixes_internal<W: Write>(
    mut writer: W,
    findings: &[CloneFinding],
    all_files: &[(PathBuf, String)],
    dry_run: bool,
    #[allow(unused_variables)] with_cst: bool,
) -> Result<()> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    if dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Would apply the following fixes:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying fixes...".cyan())?;
    }

    let mut edits_by_file: HashMap<PathBuf, Vec<Edit>> = HashMap::new();
    let mut seen_ranges: HashSet<(PathBuf, usize, usize)> = HashSet::new();

    for finding in findings {
        if finding.is_duplicate && finding.fix_confidence >= 90 {
            #[allow(unused_mut)]
            let mut start_byte = finding.start_byte;
            #[allow(unused_mut)]
            let mut end_byte = finding.end_byte;

            #[cfg(feature = "cst")]
            if with_cst {
                if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &finding.file) {
                    if let Ok(mut parser) = CstParser::new() {
                        if let Ok(tree) = parser.parse(content) {
                            let mapper = AstCstMapper::new(tree);
                            let (s, e) = mapper.precise_range_for_def(start_byte, end_byte);
                            start_byte = s;
                            end_byte = e;
                        }
                    }
                }
            }

            let range_key = (finding.file.clone(), start_byte, end_byte);
            if seen_ranges.contains(&range_key) {
                continue;
            }
            seen_ranges.insert(range_key);

            if dry_run {
                writeln!(
                    writer,
                    "  Would remove {} (lines {}-{}, bytes {}-{}) from {}",
                    finding.name.as_deref().unwrap_or("<anonymous>"),
                    finding.line,
                    finding.end_line,
                    start_byte,
                    end_byte,
                    finding.file.display()
                )?;
            } else {
                edits_by_file
                    .entry(finding.file.clone())
                    .or_default()
                    .push(Edit::delete(start_byte, end_byte));
            }
        }
    }

    if !dry_run {
        for (file_path, edits) in edits_by_file {
            if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &file_path) {
                let mut rewriter = ByteRangeRewriter::new(content.clone());
                rewriter.add_edits(edits);
                if let Ok(fixed_content) = rewriter.apply() {
                    fs::write(&file_path, fixed_content)?;
                    writeln!(writer, "  {} {}", "Fixed:".green(), file_path.display())?;
                }
            }
        }
    }
    Ok(())
}
