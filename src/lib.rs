
extern crate specs;
//#[macro_use]
//extern crate specs_derive;
extern crate shred;
#[macro_use]
extern crate shred_derive;

use std::marker::PhantomData;
use std::default::Default;
use std::collections::HashMap;
use specs::{Component, Entities, Entity, FlaggedStorage, FetchMut, Join, System, ReadStorage, UnprotectedStorage};

pub struct Hierarchy<P, I> {
    sorted: Vec<Entity>,
    changed: Vec<Entity>,
    _phantom: PhantomData<(P, I)>,
}

impl<P, I> Default for Hierarchy<P, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P, I> Hierarchy<P, I> {
    pub fn new() -> Self {
        Hierarchy {
            sorted: Vec::new(),
            changed: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn all(&self) -> &[Entity] {
        self.sorted.as_slice()
    }

    pub fn changed(&self) -> &[Entity] {
        self.sorted.as_slice()
    }

    pub fn clear_changes(&mut self) {
        self.changed.clear();
    }
}

pub trait Parent {
    fn parent_entity(&self) -> Entity;
}

#[derive(SystemData)]
pub struct ParentData<'a, P, I>
where
    I: UnprotectedStorage<P> + Send + Sync + 'static,
    P: Component<Storage = FlaggedStorage<P, I>> + Parent + Send + Sync,
{
    entities: Entities<'a>,
    parents: ReadStorage<'a, P>,
    hierarchy: FetchMut<'a, Hierarchy<P, I>>,
    _phantom: PhantomData<I>,
}

pub struct HierarchySystem<P, I> {
    /// Index of an entity in the hierarchy.
    indices: HashMap<Entity, usize>,

    /// Index of the earliest child of an entity.
    earliest: HashMap<Entity, usize>,

    _phantom: PhantomData<(P, I)>,
}

impl<P, I> HierarchySystem<P, I> {
    pub fn new() -> Self {
        HierarchySystem {
            indices: HashMap::default(),
            earliest: HashMap::default(),
            _phantom: PhantomData,
        }
    }
}

impl<P, I> Default for HierarchySystem<P, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, I, P> System<'a> for HierarchySystem<P, I>
where
    I: UnprotectedStorage<P> + Send + Sync + 'static,
    P: Component<Storage = FlaggedStorage<P, I>> + Parent + Send + Sync,
{
    type SystemData = ParentData<'a, P, I>;
    fn run(&mut self, data: Self::SystemData) {
        #[cfg(feature = "profiler")]
        profile_scope!("hierarchy_system");

        let ParentData { entities, parents, mut hierarchy, _phantom } = data;
        let mut iter = (&*entities, (&parents).open().1).join();

        // TODO: Filter out dead entities.

        hierarchy.clear_changes();

        // iterate over all entities with modified parents
        while let Some((entity, parent)) = iter.next() {
            let parent_entity = parent.parent_entity();

            let this_index = *self.indices.entry(entity).or_insert_with(|| {
                hierarchy.sorted.push(entity);
                hierarchy.sorted.len() - 1
            });

            let parent_index = {
                // if the parent had a parent inserted this frame
                // then it could not be in the list
                if parents.get(parent_entity).is_some() {
                    let parent_index = self.indices.entry(parent_entity).or_insert_with(|| {
                        hierarchy.sorted.push(parent_entity);
                        hierarchy.sorted.len() - 1
                    });
                    Some(*parent_index)
                }
                else {
                    self.indices.get(&parent_entity).cloned()
                }
            };

            if let Some(parent_index) = parent_index {
                let mut earliest_index = *self.earliest.entry(parent_entity).or_insert(this_index);

                if earliest_index > this_index {
                    // this index is the earliest child
                    earliest_index = this_index;
                    self.earliest.insert(parent_entity, earliest_index);
                }

                let earliest_entity = hierarchy.sorted[earliest_index];

                if parent_index > earliest_index {
                    // swap with earliest child
                    hierarchy.sorted.swap(parent_index, earliest_index); 
                    self.indices.insert(parent_entity, earliest_index);
                    self.indices.insert(earliest_entity, parent_index);
                    self.earliest.insert(parent_entity, parent_index);
                }
            }
        }
    }
}



