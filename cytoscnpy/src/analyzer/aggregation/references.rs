//! Reference-count map helpers.

use rustc_hash::{FxHashMap, FxHashSet};

/// Propagates import binding references through `map` using `bindings`.
///
/// For every symbol in `map` with a non-zero count, follows the binding chain
/// through `bindings` and ensures all transitive sources have count ≥ 1.
pub(super) fn propagate_import_bindings(
    map: &mut FxHashMap<String, usize>,
    bindings: &FxHashMap<String, String>,
) {
    // Used symbols already count ≥ 1, so only collect the sources they
    // reach. Borrowing from `bindings` avoids cloning every symbol.
    let mut sources: FxHashSet<&str> = FxHashSet::default();
    for (name, count) in map.iter() {
        if *count == 0 {
            continue;
        }
        let mut symbol = name.as_str();
        while let Some(source) = bindings.get(symbol) {
            if !sources.insert(source) {
                break;
            }
            symbol = source;
        }
    }

    for source in sources {
        if let Some(count) = map.get_mut(source) {
            *count = (*count).max(1);
        } else {
            map.insert(source.to_owned(), 1);
        }
    }
}

/// Adds `count` to `name`, allocating a key only for a new name.
pub(super) fn add_reference(map: &mut FxHashMap<String, usize>, name: &str, count: usize) {
    if let Some(total) = map.get_mut(name) {
        *total += count;
    } else {
        map.insert(name.to_owned(), count);
    }
}
