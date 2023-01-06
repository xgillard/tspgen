use ddo::{Problem, Variable, Decision, Relaxation, StateRanking};
use smallbitset::Set64;

use crate::instance::Instance;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TspState {
    depth:       usize,
    current:     Set64,
    must_visit:  Set64,
    might_visit: Set64,
}

#[derive(Debug, Clone)]
pub struct TspModel {
    pub instance: Instance,
}

impl Problem for TspModel {
    type State = TspState;

    fn nb_variables(&self) -> usize {
        self.instance.destinations.len()
    }

    fn initial_state(&self) -> Self::State {
        let mut must = Set64::empty();
        for i in 0..self.nb_variables() {
            must = must.insert(i as u8);
        }

        TspState {
            depth: 0,
            current: Set64::singleton(0),
            must_visit: must,
            might_visit: Set64::empty(),
        }
    }

    fn initial_value(&self) -> isize {
        0
    }

    fn transition(&self, state: &Self::State, decision: ddo::Decision) -> Self::State {
        TspState{
            depth       : state.depth + 1,
            current     : Set64::singleton(decision.value as u8),
            must_visit  : state.must_visit.remove(decision.value as u8),
            might_visit : state.might_visit.remove(decision.value as u8),
        }
    }

    fn transition_cost(&self, state: &Self::State, decision: ddo::Decision) -> isize {
        let to = decision.value as usize;
        state.current.iter()
            .map(|from| self.instance.distances[from as usize][to])
            .map(|cost| (cost * 100_000.0).round() as isize)
            .min()
            .map(|v| -v) // it is a minimization problem
            .unwrap_or(0)
    }

    fn next_variable(&self, next_layer: &mut dyn Iterator<Item = &Self::State>)
        -> Option<ddo::Variable> {
        next_layer.next()
            .map(|s| s.depth)
            .filter(|d| *d < self.nb_variables())
            .map(Variable)
    }

    fn for_each_in_domain(&self, var: ddo::Variable, state: &Self::State, f: &mut dyn ddo::DecisionCallback) {
        let dest = state.must_visit.union(state.might_visit);
        if dest.len() == 1 {
            f.apply(Decision{variable: var, value: 0});
        } else {
            for to in dest.iter() {
                if to == 0 {continue;}
                
                f.apply(Decision{variable: var, value: to as isize});
            }
        }
    }
}

pub struct TspRelax;

impl Relaxation for TspRelax {
    type State = TspState;

    fn merge(&self, states: &mut dyn Iterator<Item = &Self::State>) -> Self::State {
        let mut depth = 0;
        let mut curr  = Set64::empty();
        let mut must  = Set64::full();
        let mut might = Set64::empty();

        for state in states {
            depth = depth.max(state.depth);
            curr  = curr.union(state.current);
            must  = must.inter(state.must_visit);
            might = might.union(state.might_visit); 
        }

        TspState {
            depth,
            current: curr,
            must_visit: must,
            might_visit: might.diff(must)
        }
    }

    fn relax(
        &self,
        _: &Self::State,
        _: &Self::State,
        _: &Self::State,
        _: Decision,
        cost: isize,
    ) -> isize {
        cost
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TspRanking;
impl StateRanking for TspRanking {
    type State = TspState;

    fn compare(&self, a: &Self::State, b: &Self::State) -> std::cmp::Ordering {
        a.must_visit.len().cmp(&b.must_visit.len())
            .then_with(|| a.might_visit.len().cmp(&b.might_visit.len()))
            .then_with(|| a.current.len().cmp(&b.current.len()))
            .reverse()
    }
}