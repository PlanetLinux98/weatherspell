// The Manage Locations dialog's working copy of the saved locations
// (LocationEditor.cs in 0.1). Every edit stays here until commit, so
// Cancel undoes all of them and Remove needs no "are you sure". An entry
// remembers which saved location it came from, and commit starts from
// that one as it is then, so the seen alert ids and the last UTC offset
// move with it even when an alert check updated them while the dialog was
// open; only the nickname and the notify switch are staged here.

use std::fmt;

use crate::location::Location;
use crate::settings::SavedLocation;

#[derive(Clone, Debug, PartialEq)]
pub struct LocationEntry {
    // Its index in the list the editor was opened with; None for a place
    // added in the dialog.
    pub original: Option<usize>,
    pub place: Location,
    pub nickname: Option<String>,
    pub notify_alerts: bool,
}

impl LocationEntry {
    pub fn display_name(&self) -> String {
        match &self.nickname {
            Some(nickname) => nickname.clone(),
            None => self.place.full_name(),
        }
    }
}

// "Home (Peterborough, Ontario, Canada)": the full name stays in view
// behind a nickname. A location whose alerts are not spoken says so, so
// arrowing through the list tells everything the dialog holds.
impl fmt::Display for LocationEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.nickname {
            Some(nickname) => write!(f, "{nickname} ({})", self.place.full_name())?,
            None => f.write_str(&self.place.full_name())?,
        }
        if !self.notify_alerts {
            f.write_str("; no alert notifications")?;
        }
        Ok(())
    }
}

pub struct LocationEditor {
    entries: Vec<LocationEntry>,
}

impl LocationEditor {
    pub fn new(saved: &[SavedLocation]) -> LocationEditor {
        let entries = saved
            .iter()
            .enumerate()
            .map(|(i, s)| LocationEntry {
                original: Some(i),
                place: s.to_location(),
                nickname: s.nickname.clone(),
                notify_alerts: s.notify_alerts,
            })
            .collect();
        LocationEditor { entries }
    }

    pub fn entries(&self) -> &[LocationEntry] {
        &self.entries
    }

    // False at either end of the list, where there is nowhere to go.
    pub fn move_by(&mut self, index: usize, by: isize) -> bool {
        match index.checked_add_signed(by) {
            Some(to) if index < self.entries.len() && to < self.entries.len() => {
                let entry = self.entries.remove(index);
                self.entries.insert(to, entry);
                true
            }
            _ => false,
        }
    }

    pub fn remove(&mut self, index: usize) {
        self.entries.remove(index);
    }

    // A place already in the list is not added twice: its index comes
    // back with false.
    pub fn add(&mut self, place: Location) -> (usize, bool) {
        if let Some(existing) = self
            .entries
            .iter()
            .position(|e| e.place.is_same_place(&place))
        {
            return (existing, false);
        }
        self.entries.push(LocationEntry {
            original: None,
            place,
            nickname: None,
            notify_alerts: true,
        });
        (self.entries.len() - 1, true)
    }

    // Blank goes back to the full name.
    pub fn rename(&mut self, index: usize, nickname: &str) {
        let nickname = nickname.trim();
        self.entries[index].nickname = (!nickname.is_empty()).then(|| nickname.to_string());
    }

    pub fn set_notify(&mut self, index: usize, notify: bool) {
        self.entries[index].notify_alerts = notify;
    }

    // The saved list as edited, from the saved list as it is now: the
    // locations that were there as they are now, plus the nickname and
    // notify switch set here; new ones for places added.
    pub fn commit(&self, saved: &[SavedLocation]) -> Vec<SavedLocation> {
        self.entries
            .iter()
            .map(|e| {
                let mut s = e
                    .original
                    .and_then(|i| saved.get(i))
                    .cloned()
                    .unwrap_or_else(|| SavedLocation::from_location(&e.place));
                s.nickname = e.nickname.clone();
                s.notify_alerts = e.notify_alerts;
                s
            })
            .collect()
    }

    // Which location of the committed list to show, given the index of the
    // one that was on screen when the editor opened: that one, wherever it
    // has moved to; if it was removed, the one that now stands where it
    // stood (or the last, if it stood at the end); None when none are left.
    pub fn shown(&self, before: Option<usize>) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        before
            .and_then(|b| self.entries.iter().position(|e| e.original == Some(b)))
            .or(Some(before.unwrap_or(0).min(self.entries.len() - 1)))
    }
}
