use super::*;

const PASSWD_FILE: &str = "root:x:0:0:root:/root:/bin/bash\n\
    me:x:1000:1000:Me:/nonexistent:/bin/bash\n\
    other:x:1001:1001:Other:/nonexistent:/bin/bash\n\
    shared:x:1002:100:Shared:/nonexistent:/bin/bash\n";

#[test]
fn a_group_with_no_listed_members_that_is_the_primary_group_is_private() {
    let group = "me:x:1000:\nother:x:1001:\n";
    assert!(is_private_group(PASSWD_FILE, group, 1000, 1000, 1000));
}

#[test]
fn the_caller_listed_in_their_own_group_is_still_private() {
    let group = "me:x:1000:me\n";
    assert!(is_private_group(PASSWD_FILE, group, 1000, 1000, 1000));
}

#[test]
fn a_group_that_is_not_the_primary_group_is_refused() {
    let group = "me:x:1000:\nstaff:x:50:me\n";
    assert!(!is_private_group(PASSWD_FILE, group, 1000, 1000, 50));
}

#[test]
fn another_listed_member_makes_the_group_shared() {
    let group = "me:x:1000:other\n";
    assert!(!is_private_group(PASSWD_FILE, group, 1000, 1000, 1000));
}

#[test]
fn another_account_with_the_group_as_primary_makes_it_shared() {
    let passwd = format!("{PASSWD_FILE}guest:x:1003:1000::/nonexistent:/bin/sh\n");
    let group = "me:x:1000:\n";
    assert!(!is_private_group(&passwd, group, 1000, 1000, 1000));
}

#[test]
fn a_shared_primary_group_is_refused_even_with_no_listed_members() {
    let passwd = format!("{PASSWD_FILE}mate:x:1004:100::/nonexistent:/bin/sh\n");
    let group = "users:x:100:\n";
    assert!(!is_private_group(&passwd, group, 1002, 100, 100));
}

#[test]
fn a_caller_missing_from_the_account_file_refuses_any_listed_member() {
    let group = "me:x:1000:someone\n";
    assert!(!is_private_group(
        "root:x:0:0::/:/bin/sh\n",
        group,
        1000,
        1000,
        1000
    ));
}
