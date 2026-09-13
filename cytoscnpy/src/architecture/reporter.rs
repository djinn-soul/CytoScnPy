//! Terminal and JSON reporters for module architecture results.

use super::types::ArchitectureGraphResult;
use colored::Colorize;
use std::io::Write;

/// Formats and prints the architecture analysis report to the writer.
pub fn print_terminal_report<W: Write>(
    result: &ArchitectureGraphResult,
    cycles_only: bool,
    writer: &mut W,
) -> std::io::Result<()> {
    let stats = &result.stats;

    writeln!(
        writer,
        "\n{}",
        "Python Module Architecture & Import Graph Analysis"
            .bold()
            .cyan()
    )?;
    writeln!(writer, "{}\n", "=".repeat(60).dimmed())?;

    if !cycles_only {
        print_summary_metrics(stats, writer)?;
    }

    print_cycles_section(stats, writer)?;

    if !cycles_only {
        if !stats.god_modules.is_empty() {
            print_god_modules_section(stats, writer)?;
        }
        if !stats.bidirectional_group_deps.is_empty() {
            print_bidirectional_section(stats, writer)?;
        }
        print_coupling_rankings(result, writer)?;
    }

    writeln!(writer)?;
    Ok(())
}

fn print_summary_metrics<W: Write>(
    stats: &super::types::ArchitectureGraphStats,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "{}", "Repository Metrics:".bold().underline())?;
    writeln!(
        writer,
        "  • Total Modules:       {}",
        stats.total_modules.to_string().yellow()
    )?;
    writeln!(
        writer,
        "  • Internal Edges:      {}",
        stats.total_internal_edges.to_string().yellow()
    )?;
    writeln!(
        writer,
        "  • External Packages:   {}",
        stats.total_external_imports.to_string().yellow()
    )?;
    writeln!(
        writer,
        "  • Avg Fan-In (dep on): {:.1} (max: {} in {})",
        stats.avg_fan_in,
        stats.max_fan_in,
        stats.max_fan_in_module.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "  • Avg Fan-Out (deps):  {:.1} (max: {} in {})",
        stats.avg_fan_out,
        stats.max_fan_out,
        stats.max_fan_out_module.as_deref().unwrap_or("none")
    )?;
    writeln!(writer)?;
    Ok(())
}

fn print_cycles_section<W: Write>(
    stats: &super::types::ArchitectureGraphStats,
    writer: &mut W,
) -> std::io::Result<()> {
    if stats.circular_dependency_count == 0 {
        writeln!(
            writer,
            "{} {}",
            "✓".green().bold(),
            "No circular module dependencies detected.".green()
        )?;
        return Ok(());
    }

    writeln!(
        writer,
        "{} Found {} circular dependency component(s) (largest cycle: {} modules):",
        "✖".red().bold(),
        stats.circular_dependency_count.to_string().red().bold(),
        stats.largest_cycle_size.to_string().red().bold()
    )?;

    for (idx, cycle) in stats.cycles.iter().enumerate() {
        let impact_tag = if cycle.has_runtime_impact {
            " [CRITICAL: top-level runtime imports]".red().bold()
        } else {
            " [Guarded: function-level or TYPE_CHECKING]".yellow()
        };

        writeln!(
            writer,
            "\n  {} Cycle #{} ({} modules){}:",
            "•".red(),
            idx + 1,
            cycle.modules.len().saturating_sub(1),
            impact_tag
        )?;

        for step in &cycle.steps {
            writeln!(writer, "      {}", step.dimmed())?;
        }
    }
    writeln!(writer)?;
    Ok(())
}

fn print_god_modules_section<W: Write>(
    stats: &super::types::ArchitectureGraphStats,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{} Found {} potential God Module(s) (high coupling concentration):",
        "⚠".yellow().bold(),
        stats.god_modules.len().to_string().yellow().bold()
    )?;

    for god in &stats.god_modules {
        writeln!(
            writer,
            "  • {} (fan-in: {}, fan-out: {})",
            god.module_name.bold(),
            god.fan_in.to_string().yellow(),
            god.fan_out.to_string().yellow()
        )?;
        writeln!(writer, "    {}", god.reason.dimmed())?;
    }
    writeln!(writer)?;
    Ok(())
}

fn print_bidirectional_section<W: Write>(
    stats: &super::types::ArchitectureGraphStats,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{} Found {} bidirectional package dependency pair(s):",
        "⚠".yellow().bold(),
        stats
            .bidirectional_group_deps
            .len()
            .to_string()
            .yellow()
            .bold()
    )?;

    for dep in &stats.bidirectional_group_deps {
        writeln!(
            writer,
            "  • {} ({}) ⇄ {} ({})",
            dep.group_a.cyan().bold(),
            dep.edges_a_to_b,
            dep.group_b.cyan().bold(),
            dep.edges_b_to_a
        )?;
        writeln!(
            writer,
            "      e.g. {} AND {}",
            dep.sample_a_to_b.dimmed(),
            dep.sample_b_to_a.dimmed()
        )?;
    }
    writeln!(writer)?;
    Ok(())
}

fn print_coupling_rankings<W: Write>(
    result: &ArchitectureGraphResult,
    writer: &mut W,
) -> std::io::Result<()> {
    // Top 5 Fan-In
    let mut fan_in_list: Vec<(&String, &usize)> = result.fan_in_map.iter().collect();
    fan_in_list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    fan_in_list.truncate(5);

    writeln!(
        writer,
        "{}",
        "Top Depended-On Modules (Highest Fan-In):".bold()
    )?;
    for (name, count) in fan_in_list {
        if *count > 0 {
            writeln!(writer, "  • {name:<40} imported by {count:>3} module(s)")?;
        }
    }

    // Top 5 Fan-Out
    let mut fan_out_list: Vec<(&String, &usize)> = result.fan_out_map.iter().collect();
    fan_out_list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    fan_out_list.truncate(5);

    writeln!(
        writer,
        "\n{}",
        "Top Dependency Consumers (Highest Fan-Out):".bold()
    )?;
    for (name, count) in fan_out_list {
        if *count > 0 {
            writeln!(
                writer,
                "  • {name:<40} imports {count:>3} internal module(s)"
            )?;
        }
    }
    Ok(())
}

/// Serializes the architecture graph result to formatted JSON.
pub fn print_json_report<W: Write>(
    result: &ArchitectureGraphResult,
    writer: &mut W,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writeln!(writer, "{json}")
}
