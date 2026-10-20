use super::*;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn run(exec: &str, arguments: &[&str]) -> Vec<String> {
    let arguments = strings(arguments);
    expand(
        exec,
        &ExecContext {
            arguments: &arguments,
            icon: None,
            name: "App Name",
            file: Some("/usr/share/applications/app.desktop"),
        },
    )
}

#[test]
fn a_url_code_takes_every_file_and_vanishes_without_files() {
    let exec = "gnome-text-editor --gapplication-service %U";
    assert_eq!(
        run(exec, &["/a.txt", "/b.txt"]),
        strings(&[
            "gnome-text-editor",
            "--gapplication-service",
            "/a.txt",
            "/b.txt"
        ])
    );
    assert_eq!(
        run(exec, &[]),
        strings(&["gnome-text-editor", "--gapplication-service"])
    );
}

#[test]
fn a_quoted_path_stays_one_word_and_an_escaped_quote_survives() {
    assert_eq!(
        run("\"/opt/My App/run\" --flag %f", &[]),
        strings(&["/opt/My App/run", "--flag"])
    );
    assert_eq!(
        run("tool \"say \\\"hi\\\"\"", &[]),
        strings(&["tool", "say \"hi\""])
    );
}

#[test]
fn a_percent_pair_is_one_literal_percent_and_never_a_field_code() {
    assert_eq!(run("tool 100%%f", &[]), strings(&["tool", "100%f"]));
}

#[test]
fn the_icon_code_needs_an_icon() {
    let with = expand(
        "tool %i",
        &ExecContext {
            arguments: &[],
            icon: Some("org.app"),
            name: "n",
            file: None,
        },
    );
    assert_eq!(with, strings(&["tool", "--icon", "org.app"]));
    assert_eq!(run("tool %i", &[]), strings(&["tool"]));
}

#[test]
fn the_name_and_file_codes_fill_in_place() {
    assert_eq!(
        run("tool --title=%c %k", &[]),
        strings(&[
            "tool",
            "--title=App Name",
            "/usr/share/applications/app.desktop"
        ])
    );
}

#[test]
fn deprecated_and_unknown_codes_vanish() {
    assert_eq!(
        run("tool %d %D %n %N %v %m %z a", &[]),
        strings(&["tool", "a"])
    );
}

#[test]
fn a_line_without_a_file_code_appends_options_then_operands() {
    assert_eq!(
        run("tool --serve", &["--port=1", "/a.txt"]),
        strings(&["tool", "--serve", "--port=1", "/a.txt"])
    );
}

#[test]
fn a_multi_file_code_substitutes_all_three_in_its_place() {
    assert_eq!(
        run("viewer %F --tail", &["/1", "/2", "/3"]),
        strings(&["viewer", "/1", "/2", "/3", "--tail"])
    );
    assert_eq!(
        run("viewer %U --tail", &["/1"]),
        strings(&["viewer", "/1", "--tail"])
    );
}

#[test]
fn a_one_file_code_loses_no_operand() {
    assert_eq!(
        run("viewer %f", &["/1", "/2"]),
        strings(&["viewer", "/1", "/2"])
    );
}

#[test]
fn the_callers_arguments_keep_their_order_so_an_option_keeps_its_value() {
    assert_eq!(
        run(
            "chromium %U",
            &["--user-data-dir", "/tmp/p", "--new-window", "https://x"]
        ),
        strings(&[
            "chromium",
            "--user-data-dir",
            "/tmp/p",
            "--new-window",
            "https://x"
        ])
    );
    assert_eq!(
        run("tool --serve", &["--port=1", "/a.txt", "--flag"]),
        strings(&["tool", "--serve", "--port=1", "/a.txt", "--flag"])
    );
}

#[test]
fn a_double_dash_and_the_operand_after_it_keep_their_place() {
    assert_eq!(
        run("viewer %F", &["--", "-odd.txt"]),
        strings(&["viewer", "--", "-odd.txt"])
    );
    assert_eq!(
        run("viewer %F", &["-a", "--", "-odd.txt"]),
        strings(&["viewer", "-a", "--", "-odd.txt"])
    );
}

#[test]
fn an_embedded_file_code_never_takes_an_operand() {
    assert_eq!(
        run("sh -c \"cat %f\"", &["; rm -rf /"]),
        strings(&["sh", "-c", "cat ", "; rm -rf /"])
    );
}

#[test]
fn an_empty_quoted_argument_is_kept() {
    assert_eq!(run("tool \"\" x", &[]), strings(&["tool", "", "x"]));
}

#[test]
fn the_program_skips_an_env_prefix_and_field_codes() {
    assert_eq!(
        program_word("env FOO=1 /opt/app/bin/tool %f").as_deref(),
        Some("/opt/app/bin/tool")
    );
    assert_eq!(
        program_word("nautilus --new-window %U").as_deref(),
        Some("nautilus")
    );
    assert_eq!(program_word("%f"), None);
    assert_eq!(program_word(""), None);
}
