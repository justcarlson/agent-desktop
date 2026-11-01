use std::path::Path;

use super::{RendererPaths, SUN_PATH_LIMIT, compositor_dir};

const SIGNATURE: &str = "4520b30d498daca8079365bdb909a8dea38e8d55_1730458291_1468916127";

#[test]
fn the_documented_runtime_dir_gives_a_seventy_four_byte_socket_path() {
    let paths = RendererPaths::for_session(Path::new("/run/user/1000"), SIGNATURE, "s8f3k2p9")
        .expect("paths");
    assert_eq!(paths.socket.as_os_str().len(), 74);
    assert_eq!(
        paths.socket.extension().and_then(|e| e.to_str()),
        Some("sock")
    );
    assert_eq!(
        paths.dir,
        compositor_dir(Path::new("/run/user/1000"), SIGNATURE)
    );
}

#[test]
fn the_longest_realistic_runtime_dir_with_a_long_signature_fits() {
    let long_signature = format!("{SIGNATURE}_{}", "f".repeat(40));
    assert!(long_signature.len() >= 60);
    let paths = RendererPaths::for_session(
        Path::new("/run/user/4294967295"),
        &long_signature,
        &"x".repeat(200),
    )
    .expect("paths");
    assert!(paths.socket.as_os_str().len() < 108);
}

#[test]
fn a_socket_path_over_the_limit_is_refused_not_truncated() {
    let runtime = format!("/tmp/{}", "r".repeat(60));
    let error = RendererPaths::for_session(Path::new(&runtime), SIGNATURE, "s8f3k2p9")
        .expect_err("a path longer than sun_path must be refused");
    let details = error.details.expect("details");
    assert_eq!(details["kind"], "cursor_socket_path_too_long");
    assert_eq!(details["limit"], SUN_PATH_LIMIT);
    assert!(details["length"].as_u64().expect("length") > SUN_PATH_LIMIT as u64);
}

#[test]
fn a_socket_path_of_exactly_the_limit_is_accepted() {
    let fixed = "/agent-desktop/cursor/0123456789abcdef/0123456789abcdef.sock".len();
    let runtime = format!("/{}", "r".repeat(SUN_PATH_LIMIT - fixed - 1));
    let paths = RendererPaths::for_session(Path::new(&runtime), SIGNATURE, "s").expect("paths");
    assert_eq!(paths.socket.as_os_str().len(), SUN_PATH_LIMIT);
    let over = format!("{runtime}r");
    assert!(RendererPaths::for_session(Path::new(&over), SIGNATURE, "s").is_err());
}

#[test]
fn sessions_and_compositors_get_distinct_files() {
    let run = Path::new("/run/user/1000");
    let one = RendererPaths::for_session(run, SIGNATURE, "a").expect("paths");
    let two = RendererPaths::for_session(run, SIGNATURE, "b").expect("paths");
    let other = RendererPaths::for_session(run, "other", "a").expect("paths");
    assert_eq!(one.dir, two.dir);
    assert_ne!(one.socket, two.socket);
    assert_ne!(one.dir, other.dir);
    assert_eq!(one.holders, one.dir.join("holders"));
}
