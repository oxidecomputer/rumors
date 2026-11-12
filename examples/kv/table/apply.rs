use std::cmp::Ordering;
use std::collections::btree_map::Entry;

impl Table {
    fn update(&mut self, version: Version, update: &Update) -> Option<Version> {
        let value = update.value.clone();
        match self.entries.entry(update.key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert((version, value));
                None
            }
            Entry::Occupied(mut entry) => match version.ranked().cmp(&entry.get().0.ranked()) {
                Ordering::Greater => Some(entry.insert((version, value)).0),
                Ordering::Equal => None,
                Ordering::Less => Some(version),
            },
        }
    }
}
