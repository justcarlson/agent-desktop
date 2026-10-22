use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

struct Roots {
    children: Vec<(String, zbus::zvariant::OwnedObjectPath)>,
    names: Arc<AtomicUsize>,
}

#[zbus::interface(name = "org.a11y.atspi.Accessible")]
impl Roots {
    fn get_children(&self) -> Vec<(String, zbus::zvariant::OwnedObjectPath)> {
        self.children.clone()
    }

    #[zbus(property)]
    fn name(&self) -> String {
        self.names.fetch_add(1, Ordering::SeqCst);
        "fixture".into()
    }
}

struct Owners;

#[zbus::interface(name = "org.freedesktop.DBus")]
impl Owners {
    fn get_name_owner(&self, _name: &str) -> String {
        ":1.registry".into()
    }

    #[zbus(name = "GetConnectionUnixProcessID")]
    fn pid(&self, name: &str) -> u32 {
        if name == ":1.other" {
            unsafe { libc::getppid() as u32 }
        } else {
            std::process::id()
        }
    }
}

struct Events;

#[zbus::interface(name = "org.a11y.atspi.Registry")]
impl Events {
    fn register_event(&self, _event: &str, _properties: Vec<String>, _app: &str) {}
}

#[test]
fn launch_window_lookup_never_enriches_an_unrelated_process() {
    let unrelated = Arc::new(AtomicUsize::new(0));
    let target = Arc::new(AtomicUsize::new(0));
    let root = |children, names| Roots { children, names };
    let (client, server) = std::os::unix::net::UnixStream::pair().unwrap();
    let (client, server) = zbus::block_on(futures_lite::future::zip(
        zbus::connection::Builder::async_io_unix_stream(client)
            .p2p()
            .build(),
        zbus::connection::Builder::async_io_unix_stream(server)
            .server(zbus::Guid::generate())
            .unwrap()
            .p2p()
            .serve_at("/org/freedesktop/DBus", Owners)
            .unwrap()
            .serve_at("/org/a11y/atspi/registry", Events)
            .unwrap()
            .serve_at(
                crate::system::REGISTRY_ROOT_PATH,
                root(
                    vec![
                        (":1.target".into(), "/target".try_into().unwrap()),
                        (":1.other".into(), "/other".try_into().unwrap()),
                    ],
                    Arc::default(),
                ),
            )
            .unwrap()
            .serve_at("/target", root(Vec::new(), target.clone()))
            .unwrap()
            .serve_at("/other", root(Vec::new(), unrelated.clone()))
            .unwrap()
            .build(),
    ));
    let _server = server.unwrap();
    let bus = crate::system::bus::tests::connected(client.unwrap());
    let deadline = Deadline::after(5_000).unwrap();
    let facts = process_facts::read(std::process::id()).unwrap();
    let owner = Running {
        name: crate::system::app::app_name::app_name(&facts),
        pid: std::process::id(),
        instance: facts.instance,
        executable: facts.executable,
    };
    let options = LaunchOptions::default();
    let lease = InteractionLease::guarded(deadline, ()).unwrap();
    let launch = Launch {
        bus: &bus,
        id: "fixture",
        options: &options,
        lease: &lease,
        deadline,
    };
    assert!(
        launch
            .window_of(&owner, false, &HashSet::new())
            .unwrap()
            .is_none()
    );
    assert_eq!(unrelated.load(Ordering::SeqCst), 0);
    assert!(target.load(Ordering::SeqCst) > 0, "the owner was enriched");
}
