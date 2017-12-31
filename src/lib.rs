
extern crate specs;
extern crate shred;
extern crate shrev;
#[macro_use]
extern crate shred_derive;

use std::marker::PhantomData;
use std::ops::DerefMut;
use std::collections::HashMap;

use specs::prelude::*;
use shrev::EventChannel;

pub struct Hierarchy<P> {
    sorted: Vec<Entity>,
    entities: HashMap<Index, usize>,

    children: HashMap<Entity, Vec<Entity>>,
    changed: EventChannel<Entity>,

    modified_id: ReaderId<ModifiedFlag>,
    inserted_id: ReaderId<InsertedFlag>,
    removed_id: ReaderId<RemovedFlag>,

    modified: BitSet,
    inserted: BitSet,
    removed: BitSet,

    _phantom: PhantomData<P>,
}

impl<P> Hierarchy<P> {
    /// Create a new hierarchy object.
    pub fn new<D>(parents: &mut Storage<P, D>) -> Self
    where
        P: Component,
        P::Storage: Tracked,
        D: DerefMut<Target = ::specs::storage::MaskedStorage<P>>,
    {
        Hierarchy {
            sorted: Vec::new(),
            entities: HashSet::new(),
            children: HashMap::new(),
            changed: EventChannel::new(),

            modified_id: parents.track_modified(),
            inserted_id: parents.track_inserted(),
            removed_id: parents.track_removed(),

            modified: BitSet::new(),
            inserted: BitSet::new(),
            removed: BitSet::new(),

            _phantom: PhantomData,
        }
    }

    /// Returns all sorted entities that contain parents.
    ///
    /// Note: This does not include entities that **are** parents.
    pub fn all(&self) -> &[Entity] {
        self.sorted.as_slice()
    }

    /// Gets the children of a specific entity.
    pub fn children(&self, entity: Entity) -> Option<&[Entity]> {
        self.children.get(&entity).map(|vec| vec.as_slice())
    }

    pub fn maintain(&mut self, data: ParentData<P>)
    where
        P: Component + Parent,
        P::Storage: Tracked,
    {
        let ParentData { entities, parents, hierarchy, .. } = data;

        // Maintain tracking
        self.modified.clear();
        self.inserted.clear();
        self.removed.clear();

        parents.populate_modified(&mut self.modified_id, &mut self.modified);
        parents.populate_inserted(&mut self.inserted_id, &mut self.inserted);
        parents.populate_removed(&mut self.removed_id, &mut self.removed);
        
        for (entity, _) in (&*entities, self.inserted & parents.mask()).join() {
            self.sorted.push(entity);
        }

        for (entity, _) in (&*entities, self.removed).join() {
            
        }
        
    }
}

pub trait Parent {
    fn parent_entity(&self) -> Entity;
}

#[derive(SystemData)]
pub struct ParentData<'a, P>
where
    P: Component + Parent,
    P::Storage: Tracked,
{
    entities: Entities<'a>,
    parents: ReadStorage<'a, P>,
    hierarchy: FetchMut<'a, Hierarchy<P>>,
}

