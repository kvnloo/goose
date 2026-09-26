use goose_core::{
    capture_correlation::{
        run_capture_correlation, CaptureCorrelationOptions, DEFAULT_MIN_OWNED_CAPTURES_PER_SUMMARY,
    },
    fixtures::{build_fixture_index, load_fixture_index},
    report::write_json_report,
    tool_args::{args, default_path, flag, optional_usize, path_value, InvalidWording},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> goose_core::GooseResult<()> {
    let args = args();
    let fixtures = default_path(&args, "--fixtures", "fixtures")?;
    let index_path = path_value(&args, "--index")?;
    let output = path_value(&args, "--output")?;
    let min_owned_captures_per_summary =
        optional_usize(&args, "--min-owned-captures", InvalidWording::Terse)?
            .unwrap_or(DEFAULT_MIN_OWNED_CAPTURES_PER_SUMMARY);
    let require_owned_captures = flag(&args, "--require-owned-captures");

    let index = match index_path {
        Some(path) => load_fixture_index(&path)?,
        None => build_fixture_index(&fixtures)?,
    };
    let report = run_capture_correlation(
        &fixtures,
        &index,
        CaptureCorrelationOptions {
            min_owned_captures_per_summary,
            require_owned_captures,
        },
    );
    write_json_report(&report, output.as_deref())?;
    if report.pass {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
