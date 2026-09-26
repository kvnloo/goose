use goose_core::{
    property_tests::{
        run_property_suite, PropertySuiteOptions, DEFAULT_CASES_PER_GROUP, DEFAULT_PROPERTY_SEED,
    },
    report::write_json_report,
    tool_args::{args, optional_u64, optional_usize, path_value, InvalidWording},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> goose_core::GooseResult<()> {
    let args = args();
    let seed =
        optional_u64(&args, "--seed", InvalidWording::Terse)?.unwrap_or(DEFAULT_PROPERTY_SEED);
    let cases_per_group =
        optional_usize(&args, "--cases", InvalidWording::Terse)?.unwrap_or(DEFAULT_CASES_PER_GROUP);
    let output = path_value(&args, "--output")?;

    let report = run_property_suite(PropertySuiteOptions {
        seed,
        cases_per_group,
    })?;
    write_json_report(&report, output.as_deref())?;
    if report.pass {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
