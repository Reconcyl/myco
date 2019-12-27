use rand::Rng;
use rand::rngs::StdRng;

use std::collections::{HashSet, BTreeMap};
use std::ops::SubAssign;

mod state;

use crate::grid::{Grid, };
use super::instruction::Instruction;

pub use state::{Response, OrganismState, get_points_for_selection};

fn dec_option<T: SubAssign + Ord + From<u8>>(opt: &mut Option<T>) -> bool {
    if let Some(t) = opt {
        if *t > T::from(0) {
            *t -= T::from(1);
            true
        } else {
            false
        }
    } else {
        true
    }
}

/// A unique identifier for an organism.
pub type OrganismId = u64;
/// The organism's index in the list of living ones.
type OrganismIdx = usize;

#[derive(Debug)]
pub struct OrganismContext {
    id: OrganismId,
    pub child_potential: Option<u8>,
    pub life_potential: Option<u8>,
    pub delay_cycles: u8,
    pub organism: OrganismState,
}

impl OrganismContext {
    pub fn id(&self) -> OrganismId {
        self.id
    }
}

pub struct OrganismCollection {
    /// The total number of organisms that have been created.
    next_id: OrganismId,
    /// The number of children an organism is permitted to have.
    pub max_children: Option<u8>,
    /// The number of cycles that an organism is permitted to live.
    pub lifetime: Option<u8>,
    /// `None` flags a dead organism.
    organisms: Vec<Option<OrganismContext>>,
    /// Mapping from IDs of living all organisms to their indices into the Vec.
    id_map: BTreeMap<OrganismId, OrganismIdx>,
    /// RNG used to determine which organism to kill.
    kill_rng: StdRng,

    // Invariants:
    // - `len` is equal to the number of elements in `OrganismContext`.
    // - id_map contains `(id, idx)` if and only if `organisms[idx].is_some()` with that `id`.
}

impl OrganismCollection {
    fn new_id(&mut self) -> OrganismId {
        let new = self.next_id;
        self.next_id += 1;
        new
    }
    fn create_context(&mut self, state: OrganismState) -> OrganismContext {
        OrganismContext {
            id: self.new_id(),
            child_potential: self.max_children,
            life_potential: self.lifetime,
            delay_cycles: 0,
            organism: state
        }
    }
    fn kill_random(&mut self) {
        if self.len() == 0 {
            panic!("nothing to kill");
        }
        loop {
            let idx = self.kill_rng.gen_range(0, self.organisms.len());
            if let Some(context) = &self.organisms[idx] {
                let id = context.id;
                self.remove(id);
                break;
            }
        }
    }
    pub fn new(kill_rng: StdRng) -> Self {
        Self {
            next_id: 0,
            max_children: Some(4),
            lifetime: Some(100),
            organisms: Vec::new(),
            id_map: BTreeMap::new(),
            kill_rng,
        }
    }
    pub fn len(&self) -> usize {
        self.id_map.len()
    }
    pub fn alive(&self, id: OrganismId) -> bool {
        self.id_map.contains_key(&id)
    }
    pub fn get(&self, id: OrganismId) -> Option<&OrganismContext> {
        let idx = *self.id_map.get(&id)?;
        self.organisms[idx].as_ref()
    }
    pub fn get_opt(&self, id: Option<OrganismId>) -> Option<&OrganismContext> {
        id.and_then(|id| self.get(id))
    }
    pub fn get_mut(&mut self, id: OrganismId) -> Option<&mut OrganismContext> {
        let idx = *self.id_map.get(&id)?;
        self.organisms[idx].as_mut()
    }
    pub fn get_opt_mut(&mut self, id: Option<OrganismId>) -> Option<&mut OrganismContext> {
        id.and_then(move |id| self.get_mut(id))
    }
    pub fn insert(&mut self, state: OrganismState) {
        let context = self.create_context(state);
        let id = context.id;
        let mut context = Some(context);
        let mut created_idx = None;
        for (idx, p) in self.organisms.iter_mut().enumerate() {
            if p.is_none() {
                // The compiler can't tell that either this block will run
                // XOR the `unwrap_or_else` block will run, so we need to
                // not move the context.
                *p = context.take();
                created_idx = Some(idx);
                break;
            }
        }
        let idx = created_idx.unwrap_or_else(|| {
            let idx = self.organisms.len();
            self.organisms.push(context);
            idx
        });
        self.id_map.insert(id, idx);
    }
    pub fn remove(&mut self, id: OrganismId) {
        let idx = self.id_map.remove(&id).unwrap();
        self.organisms.swap_remove(idx).unwrap();
        // Since the `swap_remove` call reordered the organism at the end of the array to the start,
        // we need to update its index in the map.
        if let Some(Some(replaced)) = self.organisms.get(idx) {
            *self.id_map.get_mut(&replaced.id).unwrap() = idx;
        }
    }
    pub fn iter(&self) -> impl Iterator<Item=&OrganismContext> {
        self.id_map.values()
            .filter_map(move |&idx| self.organisms[idx].as_ref())
    }
    /// Run a cycle for each organism, in arbitrary order.
    pub fn run_cycle<R: Rng>(&mut self, grid: &mut Grid<R>, max_organisms: Option<usize>) {
        let mut new = Vec::new();
        let mut suicides = Vec::new();
        for context in &mut self.organisms {
            if let Some(context) = context {
                let id = context.id;
                if context.delay_cycles != 0 {
                    context.delay_cycles -= 1;
                    continue;
                }
                if !dec_option(&mut context.life_potential) {
                    suicides.push(id);
                    continue;
                }
                // Have the organism run the instruction and then handle its response.
                let ins = Instruction::from_byte(grid[context.organism.ip]);
                match context.organism.run(grid, ins) {
                    Response::Delay(delay) => {
                        context.delay_cycles = delay;
                        context.organism.advance(grid);
                    }
                    Response::Fork(mut child) => {
                        context.organism.advance(grid);
                        if dec_option(&mut context.child_potential) {
                            child.advance(grid);
                            new.push(child);
                        }
                    }
                    Response::Die => {
                        suicides.push(id);
                    }
                }
            }
        }
        for id in suicides {
            self.remove(id);
        }
        if let Some(max) = max_organisms {
            let deaths_required = (self.len() + new.len()).saturating_sub(max);
            for _ in 0..deaths_required {
                self.kill_random();
            }
        }
        for state in new {
            self.insert(state);
        }
    }
    pub fn dedup(&mut self) {
        let mut organisms = HashSet::<(u8, OrganismState)>::new();
        for ctx_ref in &mut self.organisms {
            if let Some(ctx) = ctx_ref {
                if !organisms.insert((ctx.delay_cycles, ctx.organism.clone())) {
                    self.id_map.remove(&ctx.id);
                    *ctx_ref = None;
                }
            }
        }
    }
}