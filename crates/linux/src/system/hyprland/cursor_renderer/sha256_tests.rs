use super::{digest, short_key};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn digests_match_the_fips_vectors() {
    assert_eq!(
        hex(&digest(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(&digest(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn digests_a_message_spanning_many_blocks() {
    let million = vec![b'a'; 1_000_000];
    assert_eq!(
        hex(&digest(&million)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn short_key_is_the_first_sixteen_hex_digits() {
    assert_eq!(short_key("abc"), "ba7816bf8f01cfea");
    assert_eq!(short_key("abc").len(), 16);
}
