use goose_core::{
    perf_budget::{run_perf_budget, PerfBudgetOptions, PerfBudgets, DEFAULT_PERF_SCALE},
    report::write_json_report,
    tool_args::{args, optional_mib, optional_u64, optional_usize, path_value, InvalidWording},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> goose_core::GooseResult<()> {
    let args = args();
    let scale =
        optional_usize(&args, "--scale", InvalidWording::Terse)?.unwrap_or(DEFAULT_PERF_SCALE);
    let output = path_value(&args, "--output")?;
    let budgets = PerfBudgets {
        parser_max_duration_ms: optional_u64(&args, "--max-parser-ms", InvalidWording::Terse)?
            .unwrap_or_else(|| PerfBudgets::default().parser_max_duration_ms),
        deframer_max_duration_ms: optional_u64(&args, "--max-deframer-ms", InvalidWording::Terse)?
            .unwrap_or_else(|| PerfBudgets::default().deframer_max_duration_ms),
        algorithms_max_duration_ms: optional_u64(
            &args,
            "--max-algorithms-ms",
            InvalidWording::Terse,
        )?
        .unwrap_or_else(|| PerfBudgets::default().algorithms_max_duration_ms),
        export_max_duration_ms: optional_u64(&args, "--max-export-ms", InvalidWording::Terse)?
            .unwrap_or_else(|| PerfBudgets::default().export_max_duration_ms),
        parser_max_estimated_peak_bytes: optional_mib(
            &args,
            "--max-parser-mib",
            InvalidWording::Terse,
        )?
        .unwrap_or_else(|| PerfBudgets::default().parser_max_estimated_peak_bytes),
        deframer_max_estimated_peak_bytes: optional_mib(
            &args,
            "--max-deframer-mib",
            InvalidWording::Terse,
        )?
        .unwrap_or_else(|| PerfBudgets::default().deframer_max_estimated_peak_bytes),
        algorithms_max_estimated_peak_bytes: optional_mib(
            &args,
            "--max-algorithms-mib",
            InvalidWording::Terse,
        )?
        .unwrap_or_else(|| PerfBudgets::default().algorithms_max_estimated_peak_bytes),
        export_max_estimated_peak_bytes: optional_mib(
            &args,
            "--max-export-mib",
            InvalidWording::Terse,
        )?
        .unwrap_or_else(|| PerfBudgets::default().export_max_estimated_peak_bytes),
    };

    let report = run_perf_budget(PerfBudgetOptions { scale, budgets })?;
    write_json_report(&report, output.as_deref())?;
    if report.pass {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
