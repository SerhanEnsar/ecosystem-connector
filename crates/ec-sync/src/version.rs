//! Version vectors: which changes of each device a version includes.

use std::collections::BTreeMap;

use ec_core::DeviceId;
use serde::{Deserialize, Serialize};

/// How two versions relate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// The same version.
    Equal,
    /// The other version includes every change of this one, and more.
    Before,
    /// This version includes every change of the other one, and more.
    After,
    /// Each has changes the other lacks: they were made independently.
    Concurrent,
}

/// For each device, how many of its changes a version includes. A device that is
/// missing counts as 0.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector(BTreeMap<DeviceId, u64>);

impl VersionVector {
    /// The counter of `device`.
    pub fn get(&self, device: &DeviceId) -> u64 {
        self.0.get(device).copied().unwrap_or(0)
    }

    /// Records one more change made by `me`.
    pub fn bump(&mut self, me: DeviceId) {
        let n = self.0.entry(me).or_insert(0);
        *n = n.saturating_add(1);
    }

    /// The version that includes every change of both.
    pub fn merged(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for (device, &n) in &other.0 {
            let mine = out.0.entry(*device).or_insert(0);
            *mine = (*mine).max(n);
        }
        out
    }

    /// How this version relates to `other`.
    pub fn compare(&self, other: &Self) -> Order {
        let (mut ahead, mut behind) = (false, false);
        for device in self.0.keys().chain(other.0.keys()) {
            let (mine, theirs) = (self.get(device), other.get(device));
            ahead |= mine > theirs;
            behind |= mine < theirs;
        }
        match (ahead, behind) {
            (false, false) => Order::Equal,
            (true, false) => Order::After,
            (false, true) => Order::Before,
            (true, true) => Order::Concurrent,
        }
    }

    /// The largest counter.
    pub fn max_counter(&self) -> u64 {
        self.0.values().copied().max().unwrap_or(0)
    }

    /// Number of devices with changes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no device has made a change.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(DeviceId, u64)> for VersionVector {
    fn from_iter<I: IntoIterator<Item = (DeviceId, u64)>>(iter: I) -> Self {
        Self(iter.into_iter().filter(|(_, n)| *n > 0).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_bytes([1; 32]);
    const B: DeviceId = DeviceId::from_bytes([2; 32]);

    fn v(entries: &[(DeviceId, u64)]) -> VersionVector {
        entries.iter().copied().collect()
    }

    #[test]
    fn vectors_order() {
        assert_eq!(v(&[(A, 1)]).compare(&v(&[(A, 1)])), Order::Equal);
        assert_eq!(v(&[(A, 2)]).compare(&v(&[(A, 1)])), Order::After);
        assert_eq!(v(&[(A, 1)]).compare(&v(&[(A, 1), (B, 1)])), Order::Before);
        assert_eq!(
            v(&[(A, 2)]).compare(&v(&[(A, 1), (B, 1)])),
            Order::Concurrent
        );
        // A device that is missing counts as 0.
        assert_eq!(v(&[]).compare(&v(&[(B, 1)])), Order::Before);
        assert_eq!(v(&[(A, 0)]).compare(&v(&[])), Order::Equal);
    }

    #[test]
    fn merge_then_bump_dominates_both() {
        let a = v(&[(A, 2)]);
        let b = v(&[(A, 1), (B, 3)]);
        let mut m = a.merged(&b);
        m.bump(A);
        assert_eq!(m.compare(&a), Order::After);
        assert_eq!(m.compare(&b), Order::After);
        assert_eq!(m, v(&[(A, 3), (B, 3)]));
    }
}
