use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());

pub(super) struct Oracle {
    monitor: Child,
    log: PathBuf,
    notices: Vec<(u32, String, String, String)>,
    baseline: Option<String>,
    _serial: MutexGuard<'static, ()>,
}

fn call(args: &[&str]) -> String {
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.Notifications",
            "--object-path",
            "/org/freedesktop/Notifications",
            "--method",
        ])
        .args(args)
        .output()
        .expect("independent D-Bus oracle");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

impl Oracle {
    pub(super) fn start() -> Self {
        let serial = SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = PathBuf::from(
            std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session required"),
        );
        let log = root.join(format!("notification-monitor-{}.log", std::process::id()));
        let output = File::create(&log).unwrap();
        let errors = output.try_clone().unwrap();
        let monitor = Command::new("dbus-monitor")
            .args(["--session"])
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(errors))
            .spawn()
            .expect("private monitor");
        let mut oracle = Self {
            monitor,
            log,
            notices: Vec::new(),
            baseline: None,
            _serial: serial,
        };
        oracle.wait_log("NameLost");
        oracle.baseline = Some(oracle.pointer());
        oracle
    }

    fn wait_log(&self, text: &str) {
        self.wait_after(text, 0);
    }

    fn count(&self, text: &str) -> usize {
        fs::read_to_string(&self.log).unwrap().matches(text).count()
    }

    fn wait_after(&self, text: &str, before: usize) {
        let end = Instant::now() + Duration::from_secs(10);
        while self.count(text) <= before {
            assert!(
                Instant::now() < end,
                "monitor did not observe {text}: {}",
                fs::read_to_string(&self.log).unwrap()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub(super) fn notify(&mut self, app: &str, title: &str, body: &str) {
        let before = self.count("member=Notify");
        let reply = call(&[
            "org.freedesktop.Notifications.Notify",
            app,
            "0",
            "",
            title,
            body,
            "['open', 'Ouvrir']",
            "{}",
            "0",
        ]);
        let id = reply
            .trim()
            .strip_prefix("(uint32 ")
            .unwrap()
            .strip_suffix(",)")
            .unwrap()
            .parse()
            .unwrap();
        self.notices
            .push((id, app.into(), title.into(), body.into()));
        self.wait_after("member=Notify", before);
        self.baseline = Some(self.pointer());
    }

    fn pointer(&self) -> String {
        let script = r#"
import json, os
pointer = None
if os.environ.get('XDG_SESSION_TYPE') != 'wayland':
    import ctypes as c
    x = c.CDLL('libX11.so.6')
    x.XOpenDisplay.restype = c.c_void_p
    x.XDefaultRootWindow.argtypes = [c.c_void_p]
    x.XDefaultRootWindow.restype = c.c_ulong
    x.XQueryPointer.argtypes = [c.c_void_p, c.c_ulong, c.c_void_p, c.c_void_p,
                                c.c_void_p, c.c_void_p, c.c_void_p, c.c_void_p, c.c_void_p]
    x.XCloseDisplay.argtypes = [c.c_void_p]
    display = x.XOpenDisplay(None)
    assert display
    root, child, rx, ry, wx, wy, mask = c.c_ulong(), c.c_ulong(), *[c.c_int() for _ in range(4)], c.c_uint()
    assert x.XQueryPointer(display, x.XDefaultRootWindow(display), c.byref(root), c.byref(child),
                          c.byref(rx), c.byref(ry), c.byref(wx), c.byref(wy), c.byref(mask))
    pointer = [rx.value, ry.value]
    x.XCloseDisplay(display)
print(json.dumps(pointer))
"#;
        let output = Command::new("python3")
            .args(["-c", script])
            .output()
            .expect("independent pointer observation");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .to_owned()
    }

    pub(super) fn assert_untouched(&self) {
        let observed_state = self.pointer();
        if let Some(baseline) = &self.baseline {
            assert_eq!(&observed_state, baseline, "pointer moved");
        }
        let before = self.count("member=GetServerInformation");
        call(&["org.freedesktop.Notifications.GetServerInformation"]);
        self.wait_after("member=GetServerInformation", before);
        let observed = fs::read_to_string(&self.log).unwrap();
        assert!(
            !observed.contains("member=NotificationClosed"),
            "{observed}"
        );
        assert!(!observed.contains("member=ActionInvoked"), "{observed}");
        assert!(!observed.contains("member=CloseNotification"), "{observed}");
        assert!(!observed.contains("member=NotifyPointer"), "{observed}");
        println!("preserved daemon ids and payloads: {:?}", self.notices);
    }
}

impl Drop for Oracle {
    fn drop(&mut self) {
        let _ = self.monitor.kill();
        let _ = self.monitor.wait();
        for (id, _, _, _) in &self.notices {
            call(&[
                "org.freedesktop.Notifications.CloseNotification",
                &id.to_string(),
            ]);
        }
        let _ = fs::remove_file(&self.log);
    }
}
