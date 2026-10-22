use super::super::close_live_tests::rig::{Orphan, lease};
use super::super::{SharedBus, app_inventory, list_windows};
use agent_desktop_core::{Deadline, ErrorCode, WindowFilter};
use std::process::Command;
use std::time::{Duration, Instant};

fn deadline() -> Deadline {
    Deadline::after(10_000).unwrap()
}

fn oracle(pid: u32) -> usize {
    let output = Command::new("python3")
        .args([
            "-c",
            r#"
import gi,sys,os
gi.require_version('Atspi','2.0')
from gi.repository import Atspi
root=Atspi.get_desktop(0)
count=0
for i in range(root.get_child_count()):
    app=root.get_child_at_index(i)
    if app.get_process_id()!=int(sys.argv[1]): continue
    for j in range(app.get_child_count()):
        child=app.get_child_at_index(j)
        count+=child.get_state_set().contains(Atspi.StateType.SHOWING)
print(count)
"#,
            &pid.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn native_ready(pid: u32, kind: &str, expected: usize) -> Option<String> {
    let output = Command::new("python3")
        .env("LC_ALL", "C")
        .args([
            "-c",
            r#"
import sys,subprocess,re
try:
    ids=re.findall(r'0x[0-9a-f]+',subprocess.check_output(['xprop','-root','_NET_CLIENT_LIST'],text=True))
    owned=[xid for xid in ids if subprocess.check_output(['xprop','-id',xid,'_NET_WM_PID'],text=True).partition('=')[2].strip()==sys.argv[1]]
    if len(owned)!=int(sys.argv[3]):sys.exit(1)
    if not sys.argv[2]:sys.exit(0)
    normal=[xid for xid in owned if '_NET_WM_WINDOW_TYPE_NORMAL' in subprocess.check_output(['xprop','-id',xid,'_NET_WM_WINDOW_TYPE'],text=True)]
    if len(normal)!=1:sys.exit(1)
    xid=normal[0]
    print(xid)
    sys.exit(0)
except (subprocess.CalledProcessError,AttributeError,ValueError,IndexError):pass
sys.exit(1)
"#,
            &pid.to_string(),
            kind,
            &expected.to_string(),
        ])
        .output()
        .unwrap();
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).unwrap().trim().to_owned())
}

#[test]
#[ignore = "runs owned GTK windows; use a private managed X11 or Wayland session"]
fn native_window_structure_overrides_service_names_and_helper_labels() {
    let runtime = std::path::PathBuf::from(std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").unwrap());
    let script = runtime.join("structural-windows.py");
    std::fs::write(
        &script,
        r#"
import gi,sys,os
gi.require_version('Gtk',sys.argv[1])
from gi.repository import Gtk, Gdk, GLib
if sys.argv[1]=='3.0':
    normal=Gtk.Window()
    normal.set_default_size(290,170)
    normal.connect('destroy',Gtk.main_quit)
    if len(sys.argv)>2:
        helper=Gtk.Window()
        helper.set_type_hint(Gdk.WindowTypeHint.DOCK)
        helper.set_default_size(410,60)
        helper.show_all()
        def open_popup():
            global popups
            runtime=os.environ['AGENT_DESKTOP_PRIVATE_SESSION']
            if not os.path.exists(os.path.join(runtime,'popup-request-'+str(os.getpid()))):return True
            from gi.repository import Atk,GdkX11
            import subprocess
            popups=[]
            for widget in [normal,helper]:
                attributes=Gdk.WindowAttr()
                attributes.window_type=Gdk.WindowType.TEMP
                attributes.wclass=Gdk.WindowWindowClass.INPUT_OUTPUT
                attributes.x,attributes.y,attributes.width,attributes.height=widget.get_accessible().get_extents(Atk.CoordType.SCREEN)
                popup=Gdk.Window.new(None,attributes,Gdk.WindowAttributesType.X|Gdk.WindowAttributesType.Y)
                popup.show()
                Gdk.flush()
                subprocess.check_call(['xprop','-id',str(popup.get_xid()),'-f','_NET_WM_PID','32c','-set','_NET_WM_PID',str(os.getpid())],stdout=subprocess.DEVNULL)
                popups.append(popup)
            with open(os.path.join(runtime,'popup-'+str(os.getpid())), 'w') as f:f.write(str(popup.get_xid()))
            def close_popup():
                if not os.path.exists(os.path.join(runtime,'popup-close-'+str(os.getpid()))):return True
                for popup in popups:popup.destroy()
                return False
            GLib.timeout_add(50,close_popup)
            return False
        GLib.timeout_add(50,open_popup)
    normal.show_all()
    Gtk.main()
else:
    Gtk.init()
    normal=Gtk.Window()
    normal.set_default_size(290,170)
    normal.present()
    second=Gtk.Window()
    second.set_default_size(290,170)
    second.present()
    GLib.MainLoop().run()
"#,
    )
    .unwrap();
    let bus = SharedBus::default();
    let executables = runtime.join("window-filter-apps");
    std::fs::create_dir_all(&executables).unwrap();
    for (name, toolkit) in [("gnome-shell", "3.0"), ("mutter-x11-frames", "4.0")] {
        let executable = executables.join(name);
        std::fs::copy("/usr/bin/python3", &executable).unwrap();
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let mut args = vec![script.to_str().unwrap(), toolkit];
        if toolkit == "3.0" && !wayland {
            args.push("helper");
        }
        let app = Orphan::spawn(executable.to_str().unwrap(), &args, &[]);
        let user_windows = if toolkit == "4.0" { 2 } else { 1 };
        let expected = user_windows + usize::from(args.len() == 3);
        let until = Instant::now() + Duration::from_secs(20);
        while oracle(app.pid) < expected {
            assert!(
                Instant::now() < until,
                "the owned app never exposed its windows"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        if toolkit == "3.0" && !wayland {
            while native_ready(app.pid, "normal", expected).is_none() {
                assert!(
                    Instant::now() < until,
                    "native client population never settled"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        if !wayland {
            while native_ready(app.pid, "", expected).is_none() {
                assert!(
                    Instant::now() < until,
                    "native client population never settled"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
            let clients = super::super::x11::control::toplevels_of(app.pid, deadline())
                .ready()
                .unwrap()
                .into_iter()
                .map(|window| window.id)
                .collect();
            assert!(
                super::super::x11::client_population::complete(app.pid, clients, deadline()),
                "only a complete native client population permits process-wide association"
            );
        }
        let inventory = app_inventory(&bus, deadline()).unwrap();
        let listed = inventory
            .apps
            .into_iter()
            .find(|row| row.pid == app.pid)
            .unwrap();
        if toolkit == "3.0" && !wayland {
            std::fs::write(runtime.join(format!("popup-request-{}", app.pid)), "open").unwrap();
            while !runtime.join(format!("popup-{}", app.pid)).exists() {
                assert!(Instant::now() < until, "native popup was never created");
                std::thread::sleep(Duration::from_millis(50));
            }
            let popup =
                std::fs::read_to_string(runtime.join(format!("popup-{}", app.pid))).unwrap();
            let native = Command::new("xwininfo")
                .env("LC_ALL", "C")
                .args(["-id", &popup, "-stats"])
                .output()
                .unwrap();
            assert!(native.status.success());
            assert!(
                String::from_utf8_lossy(&native.stdout).contains("Override Redirect State: yes")
            );
            let clients = super::super::x11::control::toplevels_of(app.pid, deadline())
                .ready()
                .unwrap()
                .into_iter()
                .map(|window| window.id)
                .collect();
            assert!(!super::super::x11::client_population::complete(
                app.pid,
                clients,
                deadline()
            ));
            let refused = list_windows(
                &bus,
                &WindowFilter {
                    app: Some(listed.name.clone()),
                    focused_only: false,
                },
                deadline(),
            )
            .unwrap_err();
            assert_eq!(refused.code, ErrorCode::ActionNotSupported);
            std::fs::write(runtime.join(format!("popup-close-{}", app.pid)), "close").unwrap();
            loop {
                let clients = super::super::x11::control::toplevels_of(app.pid, deadline())
                    .ready()
                    .unwrap()
                    .into_iter()
                    .map(|window| window.id)
                    .collect();
                if super::super::x11::client_population::complete(app.pid, clients, deadline()) {
                    break;
                }
                assert!(Instant::now() < until, "native popup never closed");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        let windows = list_windows(
            &bus,
            &WindowFilter {
                app: Some(listed.name.clone()),
                focused_only: false,
            },
            deadline(),
        )
        .unwrap();
        assert_eq!(
            windows.len(),
            user_windows,
            "ordinary windows are not hidden by process name and dock windows are not user windows"
        );
        assert!(windows.iter().all(|window| window.pid.get() == app.pid));
        let ids: std::collections::HashSet<_> = windows.iter().map(|window| &window.id).collect();
        assert_eq!(ids.len(), user_windows);
        assert_eq!(
            oracle(app.pid),
            expected,
            "independent AT-SPI still sees the native helper"
        );
        if toolkit == "3.0" && !wayland {
            let xid = native_ready(app.pid, "normal", expected).unwrap();
            let native = Command::new("xprop")
                .args(["-id", &xid, "_NET_WM_WINDOW_TYPE"])
                .output()
                .unwrap();
            assert!(native.status.success());
            assert!(String::from_utf8_lossy(&native.stdout).contains("_NET_WM_WINDOW_TYPE_NORMAL"));
            let set = |kind: &str| {
                assert!(
                    Command::new("xprop")
                        .args([
                            "-id",
                            &xid,
                            "-f",
                            "_NET_WM_WINDOW_TYPE",
                            "32a",
                            "-set",
                            "_NET_WM_WINDOW_TYPE",
                            kind
                        ])
                        .status()
                        .unwrap()
                        .success()
                )
            };
            set("_AD_UNCLASSIFIED_WINDOW");
            let refused = list_windows(
                &bus,
                &WindowFilter {
                    app: Some(listed.name.clone()),
                    focused_only: false,
                },
                deadline(),
            )
            .unwrap_err();
            set("_NET_WM_WINDOW_TYPE_NORMAL");
            assert_eq!(refused.code, ErrorCode::ActionNotSupported);
            assert_eq!(
                list_windows(
                    &bus,
                    &WindowFilter {
                        app: Some(listed.name.clone()),
                        focused_only: false
                    },
                    deadline()
                )
                .unwrap()
                .len(),
                1
            );
        }
        let report = super::super::close::close_app(&bus, &listed, true, &lease(10_000)).unwrap();
        assert_eq!(
            report.disposition(),
            agent_desktop_core::DeliverySemantics::delivered_verified()
        );
        let closed = Instant::now() + Duration::from_secs(10);
        while oracle(app.pid) != 0 {
            assert!(
                Instant::now() < closed,
                "independent Atspi still sees windows after close"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!app.alive());
        if !wayland {
            let gone = Instant::now() + Duration::from_secs(5);
            while native_ready(app.pid, "", 0).is_none() {
                assert!(
                    Instant::now() < gone,
                    "native clients remained after process exit"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
            assert!(super::super::x11::client_population::complete(
                app.pid,
                Vec::new(),
                deadline()
            ));
        }
        let unknown = super::super::close::close_app(
            &bus,
            &agent_desktop_core::AppInfo {
                process_instance: None,
                ..listed
            },
            true,
            &lease(5_000),
        )
        .unwrap_err();
        assert_eq!(unknown.code, ErrorCode::InvalidArgs);
        println!(
            "independent Atspi: GTK {toolkit} pid {} exposed {expected} frames; product retained {user_windows} user windows and closed only that process",
            app.pid
        );
    }
}
