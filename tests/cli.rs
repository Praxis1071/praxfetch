use std::process::Command;

fn run_praxfetch(args: &[&str], no_color: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_praxfetch"));
    command.args(args);
    if no_color {
        command.env("NO_COLOR", "1");
    } else {
        command.env_remove("NO_COLOR");
    }
    command.output().expect("praxfetch should start")
}

#[test]
fn default_output_contains_only_the_approved_seven_lines() {
    let output = run_praxfetch(&["--no-color"], false);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 7, "unexpected output:\n{stdout}");

    const LOGO_COLUMN_WIDTH: usize = 18;
    let header = lines[0].get(LOGO_COLUMN_WIDTH..).unwrap_or("").trim_start();
    assert!(
        header.contains('@'),
        "first line should contain user@hostname: {}",
        lines[0]
    );

    let labels: Vec<_> = lines[1..]
        .iter()
        .map(|line| {
            line.get(LOGO_COLUMN_WIDTH..)
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
        })
        .collect();
    assert_eq!(
        labels,
        ["os", "host", "kernel", "uptime", "pkgs", "memory"],
        "unexpected system fields:\n{stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "ANSI escape found with --no-color"
    );
}

#[test]
fn no_color_environment_variable_disables_ansi_sequences() {
    let output = run_praxfetch(&[], true);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("output should be UTF-8");
    assert!(
        !stdout.contains('\u{1b}'),
        "ANSI escape found with NO_COLOR"
    );
}

#[test]
fn help_and_version_exit_successfully() {
    let help = run_praxfetch(&["--help"], false);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).expect("help should be UTF-8");
    assert!(help.contains("Usage:"));
    assert!(help.contains("--no-color"));

    let version = run_praxfetch(&["--version"], false);
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).expect("version should be UTF-8");
    assert!(version.starts_with("praxfetch "));
}

#[test]
fn invalid_arguments_fail_with_a_diagnostic() {
    let output = run_praxfetch(&["--not-a-real-option"], false);
    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("error output should be UTF-8");
    assert!(stderr.contains("unexpected argument"), "{stderr}");
}
