use std::fs;

const PASSWD: &str = "/etc/passwd";
const GROUP: &str = "/etc/group";

struct Account<'a> {
    name: &'a str,
    uid: u32,
    gid: u32,
}

fn accounts(passwd: &str) -> impl Iterator<Item = Account<'_>> {
    passwd.lines().filter_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let uid = fields.nth(1)?.parse().ok()?;
        let gid = fields.next()?.parse().ok()?;
        Some(Account { name, uid, gid })
    })
}

fn members_of(group: &str, gid: u32) -> Vec<&str> {
    group
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let found: u32 = fields.nth(2)?.parse().ok()?;
            let members = fields.next().unwrap_or("");
            (found == gid).then_some(members)
        })
        .flat_map(|members| members.split(',').filter(|member| !member.is_empty()))
        .collect()
}

/// Whether `file_gid` is the caller's own primary group and no other account belongs to it.
pub(crate) fn is_private_group(
    passwd: &str,
    group: &str,
    caller_uid: u32,
    caller_gid: u32,
    file_gid: u32,
) -> bool {
    if file_gid != caller_gid {
        return false;
    }
    let caller_name = accounts(passwd)
        .find(|account| account.uid == caller_uid)
        .map(|account| account.name);
    let another_primary =
        accounts(passwd).any(|account| account.gid == file_gid && account.uid != caller_uid);
    let another_member = members_of(group, file_gid)
        .into_iter()
        .any(|member| Some(member) != caller_name);
    !another_primary && !another_member
}

pub(crate) fn caller_owns_group(file_gid: u32) -> bool {
    let (Ok(passwd), Ok(group)) = (fs::read_to_string(PASSWD), fs::read_to_string(GROUP)) else {
        return false;
    };
    is_private_group(
        &passwd,
        &group,
        rustix::process::getuid().as_raw(),
        rustix::process::getgid().as_raw(),
        file_gid,
    )
}

#[cfg(test)]
#[path = "owner_group_tests.rs"]
mod tests;
