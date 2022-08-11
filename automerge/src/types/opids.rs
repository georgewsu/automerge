use itertools::Itertools;

use super::OpId;

/// A wrapper around `Vec<Opid>` which preserves the invariant that the ops are
/// in ascending order with respect to their counters and actor IDs. In order to
/// maintain this invariant you must provide a comparator function when adding
/// ops as the actor indices in an  OpId are not sufficient to order the OpIds
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct OpIds(Vec<OpId>);

impl<'a> IntoIterator for &'a OpIds {
    type Item = &'a OpId;
    type IntoIter = std::slice::Iter<'a, OpId>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl OpIds {
    pub(crate) fn new<I: Iterator<Item = OpId>, F: Fn(&OpId, &OpId) -> std::cmp::Ordering>(
        opids: I,
        cmp: F,
    ) -> Self {
        let mut inner = opids.collect::<Vec<_>>();
        inner.sort_by(cmp);
        Self(inner)
    }

    /// Add an op to this set of OpIds. The `comparator` must provide a
    /// consistent ordering between successive calls to `add`.
    pub(crate) fn add<F: Fn(&OpId, &OpId) -> std::cmp::Ordering>(
        &mut self,
        opid: OpId,
        comparator: F,
    ) {
        use std::cmp::Ordering::*;
        if self.is_empty() {
            self.0.push(opid);
            return;
        }
        let idx_and_elem = self
            .0
            .iter()
            .find_position(|an_opid| matches!(comparator(an_opid, &opid), Greater | Equal));
        if let Some((idx, an_opid)) = idx_and_elem {
            if comparator(an_opid, &opid) == Equal {
                // nothing to do
            } else {
                self.0.insert(idx, opid);
            }
        } else {
            self.0.push(opid);
        }
    }

    pub(crate) fn retain<F: Fn(&OpId) -> bool>(&mut self, f: F) {
        self.0.retain(f)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, OpId> {
        self.0.iter()
    }

    pub(crate) fn contains(&self, op: &OpId) -> bool {
        self.0.contains(op)
    }
}

#[cfg(test)]
mod tests {
    use super::{OpId, OpIds};
    use crate::ActorId;
    use proptest::prelude::*;

    fn gen_opid(actors: Vec<ActorId>) -> impl Strategy<Value = OpId> {
        (0..actors.len()).prop_flat_map(|actor_idx| {
            (Just(actor_idx), 0..u64::MAX).prop_map(|(actor_idx, counter)| OpId(counter, actor_idx))
        })
    }

    fn scenario() -> impl Strategy<Value = (Vec<ActorId>, Vec<OpId>)> {
        let actors = vec![
            "aaaa".try_into().unwrap(),
            "cccc".try_into().unwrap(),
            "bbbb".try_into().unwrap(),
        ];
        proptest::collection::vec(gen_opid(actors.clone()), 0..100)
            .prop_map(move |opids| (actors.clone(), opids))
    }

    proptest! {
        #[test]
        fn test_sorted_opids((actors, opids) in scenario()) {
            let mut sorted_opids = OpIds::default();
            for opid in &opids {
                sorted_opids.add(*opid, |left, right| cmp(&actors, left, right));
            }
            let result = sorted_opids.into_iter().cloned().collect::<Vec<_>>();
            let mut expected = opids;
            expected.sort_by(|left, right| cmp(&actors, left, right));
            expected.dedup();
            assert_eq!(result, expected);
        }
    }

    fn cmp(actors: &[ActorId], left: &OpId, right: &OpId) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (left, right) {
            (OpId(0, _), OpId(0, _)) => Ordering::Equal,
            (OpId(0, _), OpId(_, _)) => Ordering::Less,
            (OpId(_, _), OpId(0, _)) => Ordering::Greater,
            (OpId(a, x), OpId(b, y)) if a == b => actors[*x].cmp(&actors[*y]),
            (OpId(a, _), OpId(b, _)) => a.cmp(b),
        }
    }
}