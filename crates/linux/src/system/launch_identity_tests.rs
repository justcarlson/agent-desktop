use super::resolve;
use super::tests::fixture;
use crate::system::launch_attach::{Decision, decide};
use crate::system::running_app::Running;

fn real_process(program: &str, args: &[&str]) -> (std::process::Child, Running) {
    let child = std::process::Command::new(program)
        .args(args)
        .spawn()
        .expect("child");
    let expected = std::fs::canonicalize(program).expect("program");
    let limit = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::fs::canonicalize(format!("/proc/{}/exe", child.id()))
        .ok()
        .as_ref()
        != Some(&expected)
    {
        assert!(std::time::Instant::now() < limit, "exec did not complete");
        std::thread::yield_now();
    }
    let facts = crate::system::process_facts::read(child.id()).expect("process facts");
    let owner = Running {
        executable: facts.executable,
        pid: child.id(),
        instance: facts.instance,
        name: program.rsplit('/').next().unwrap().to_owned(),
    };
    (child, owner)
}

fn rejects_wrong_process(class: &str) {
    let fixture = fixture(&[], &[]);
    let bin = &fixture.search.path_dirs[0];
    let a = bin.join("a");
    let b = bin.join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let target = b.join("editor");
    let other = a.join("editor");
    std::fs::copy("/bin/sleep", &target).unwrap();
    std::fs::copy("/bin/sleep", &other).unwrap();
    let (mut child, mut owner) = real_process(other.to_str().unwrap(), &["30"]);
    let id = match class {
        "comm" => {
            owner.name = "unrelated".into();
            target.to_str().unwrap().to_owned()
        }
        "alias" => {
            owner.name = "sole-app".into();
            std::fs::write(
                fixture.search.data_dirs[0].join("sole-app.desktop"),
                format!(
                    "[Desktop Entry]\nType=Application\nName=Sole App\nExec={}\n",
                    target.display()
                ),
            )
            .unwrap();
            "sole-app".into()
        }
        "interpreter" => {
            child.kill().unwrap();
            child.wait().unwrap();
            let pair = real_process("/usr/bin/python3", &["-c", "import time; time.sleep(30)"]);
            child = pair.0;
            owner = pair.1;
            std::fs::write(fixture.search.data_dirs[0].join("sole-app.desktop"), "[Desktop Entry]\nType=Application\nName=Sole App\nExec=/usr/bin/python3 /opt/other.py\nTryExec=/bin/sleep\n").unwrap();
            "sole-app".into()
        }
        _ => target.to_str().unwrap().to_owned(),
    };
    let index = crate::system::alias_cache::AliasCache::new().index(&fixture.search.data_dirs);
    let plan = resolve(&id, &[], &fixture.search, &index).expect("resolve");
    let actual = decide(&plan.identity, &[owner], true, false);
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(actual, Decision::Spawn, "wrong-process class {class}");
}

#[test]
fn launch_rejects_comm_identity_without_executable_proof() {
    rejects_wrong_process("comm");
}
#[test]
fn launch_rejects_sole_entry_alias_without_executable_proof() {
    rejects_wrong_process("alias");
}
#[test]
fn launch_rejects_sole_interpreter_with_different_argv() {
    rejects_wrong_process("interpreter");
}
#[test]
fn launch_rejects_same_basename_at_another_absolute_path() {
    rejects_wrong_process("absolute");
}

#[test]
fn launch_rejects_a_different_interpreter_invocation_name() {
    let fixture = fixture(&[], &[]);
    let bin = &fixture.search.path_dirs[0];
    let requested = bin.join("bash");
    let other = bin.join("sh");
    for path in [&requested, &other] {
        std::os::unix::fs::symlink("/bin/bash", path).unwrap();
    }
    let args = ["-c", "while :; do :; done"];
    let (mut child, owner) = real_process(other.to_str().unwrap(), &args);
    std::fs::write(
        fixture.search.data_dirs[0].join("shell-app.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Shell App\nExec={} -c \"while :; do :; done\"\n",
            requested.display()
        ),
    )
    .unwrap();
    let index = crate::system::alias_cache::AliasCache::new().index(&fixture.search.data_dirs);
    let plan = resolve("shell-app", &[], &fixture.search, &index).unwrap();
    let wrong = decide(&plan.identity, std::slice::from_ref(&owner), true, false);
    let aliases = index.names_for(owner.executable.as_ref().unwrap());
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(wrong, Decision::Spawn);
    assert!(aliases.is_empty());
    let (mut child, owner) = real_process(requested.to_str().unwrap(), &args);
    let correct = decide(&plan.identity, std::slice::from_ref(&owner), true, false);
    let aliases = index.names_for(owner.executable.as_ref().unwrap());
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(correct, Decision::Attach(owner));
    assert_eq!(aliases, ["Shell App"]);
}
