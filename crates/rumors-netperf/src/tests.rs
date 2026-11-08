use clap::CommandFactory;

use super::*;

/// The command line's definition is consistent, as clap checks it: every
/// argument a conflict names, the grid's flag group included, exists.
#[test]
fn command_line_is_consistent() {
    Cli::command().debug_assert();
}

/// `run --resume` takes the run's grid, shard, and job count from its
/// directory, so it refuses each flag that would set them, and accepts the
/// flags that do not.
#[test]
fn resume_refuses_flags_the_directory_settles() {
    for flags in [
        &["--grid", "grid.toml"][..],
        &["--shared", "1e5"],
        &["--out", "elsewhere"],
        &["--jobs", "4"],
        &["--recalibrate"],
        &["--shard", "1/2"],
    ] {
        let line = ["rumors-netperf", "run", "--resume", "results"]
            .into_iter()
            .chain(flags.iter().copied());
        let error = Cli::try_parse_from(line).unwrap_err();
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "{flags:?}"
        );
    }
    let resumed = Cli::try_parse_from([
        "rumors-netperf",
        "run",
        "--resume",
        "results",
        "--allow-debug-build",
    ])
    .unwrap();
    assert!(matches!(
        resumed.command,
        Command::Run(RunArgs {
            resume: Some(_),
            ..
        })
    ));
}
