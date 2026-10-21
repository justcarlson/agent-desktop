use super::*;
use crate::tree::live_launch::Launched;

fn entry(id: &str, exec: &str, extra: &str) {
    let data = std::env::var("XDG_DATA_HOME").expect("private data");
    let entries = std::path::Path::new(&data).join("applications");
    std::fs::create_dir_all(&entries).expect("entries");
    std::fs::write(
        entries.join(format!("{id}.desktop")),
        format!("[Desktop Entry]\nType=Application\nName={id}\nExec={exec}\n{extra}"),
    )
    .expect("entry");
}

fn copy(program: &str, name: &str) -> String {
    let runtime = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session");
    let path = format!("{runtime}/{name}");
    std::fs::copy(program, &path).expect("private executable");
    path
}

#[test]
#[ignore = "starts owned interpreter apps; run inside scripts/private-session.sh --wm"]
fn shared_interpreter_launch_never_attaches_to_the_other_desktop_app() {
    let runtime = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session");
    let interpreter = copy("/usr/bin/python3", "ad-python");
    let script = format!("{runtime}/launch-identity.py");
    std::fs::write(
        &script,
        r#"
import gi, sys
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(title=sys.argv[1])
w.add(Gtk.Button(label=sys.argv[1]))
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#,
    )
    .expect("GTK script");
    entry("ad-launch-one", &format!("{interpreter} {script} one"), "");
    entry("ad-launch-two", &format!("{interpreter} {script} two"), "");
    for _ in 0..2 {
        let mut other = Launched::new();
        let other_pid = other.spawn(&interpreter, &[&script, "two"]);
        wait_until("unrelated app registry and window", || {
            registry_has(other_pid) && !x_windows_of(other_pid).is_empty()
        });
        let other_windows = x_windows_of(other_pid);
        let mut owned = Owned(Vec::new());
        let result = launch("ad-launch-one", &options(&[])).expect("start requested app");
        assert_ne!(
            result.pid.get(),
            other_pid,
            "shared interpreter is not identity"
        );
        owned.keep(result.pid.get());
        assert_owner_is_truthful(&result);
        assert!(
            xprop(&["-id", &x_windows_of(result.pid.get())[0], "_NET_WM_NAME"]).contains("one")
        );
        assert_eq!(x_windows_of(other_pid), other_windows);
        let refusal = LaunchOptions {
            attach_if_running: false,
            ..options(&[])
        };
        let error = launch(&interpreter, &refusal)
            .expect_err("explicit shared interpreter is already running");
        assert_eq!(error.code, ErrorCode::ActionFailed);
        assert_eq!(
            error.disposition,
            agent_desktop_core::DeliverySemantics::not_delivered()
        );
        println!(
            "independent X11 owners: requested {} != unrelated {other_pid}; unsafe explicit relaunch refused",
            result.pid
        );
    }
}

#[test]
#[ignore = "starts owned Zenity apps; run inside scripts/private-session.sh --wm"]
fn try_exec_launch_never_attaches_to_the_running_helper() {
    let helper = copy("/usr/bin/zenity", "ad-helper");
    let target = copy("/usr/bin/zenity", "ad-target");
    entry(
        "ad-launch-client",
        &format!("{target} --info --text=client"),
        &format!("TryExec={helper}\n"),
    );
    for _ in 0..2 {
        let mut other = Launched::new();
        let helper_pid = other.spawn(&helper, &["--info", "--text=helper"]);
        wait_until("helper registry and window", || {
            registry_has(helper_pid) && !x_windows_of(helper_pid).is_empty()
        });
        let helper_windows = x_windows_of(helper_pid);
        let mut owned = Owned(Vec::new());
        let result = launch("ad-launch-client", &options(&[])).expect("start client");
        assert_ne!(
            result.pid.get(),
            helper_pid,
            "TryExec is only an availability check"
        );
        owned.keep(result.pid.get());
        assert_owner_is_truthful(&result);
        assert_eq!(
            std::fs::read_link(format!("/proc/{}/exe", result.pid)).expect("executable oracle"),
            std::path::PathBuf::from(&target)
        );
        assert_eq!(x_windows_of(helper_pid), helper_windows);
        let refusal = LaunchOptions {
            attach_if_running: false,
            ..options(&[])
        };
        let error = launch("ad-launch-client", &refusal)
            .expect_err("proven client still refuses a fresh launch");
        assert_eq!(error.code, ErrorCode::ActionFailed);
        assert_eq!(
            error.disposition,
            agent_desktop_core::DeliverySemantics::not_delivered()
        );
        println!(
            "independent X11 owners: client {} != helper {helper_pid}; duplicate client launch refused",
            result.pid
        );
    }
}
