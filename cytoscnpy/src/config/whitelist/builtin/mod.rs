mod part1;
mod part2;
mod part3;

use super::WhitelistEntry;

pub(super) fn entries() -> Vec<WhitelistEntry> {
    let mut entries = Vec::new();
    entries.extend(part1::entries());
    entries.extend(part2::entries());
    entries.extend(part3::entries());
    entries
}
