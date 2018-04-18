extern crate hibitset;
extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate shrev;
extern crate specs;

use std::marker::PhantomData;
use std::collections::{HashMap, HashSet};

use specs::prelude::{BitSet, Component, Entities, Entity, InsertedFlag, Join, ModifiedFlag,
                     ReadStorage, ReaderId, RemovedFlag, System, SystemData, Tracked, Write,
                     WriteStorage};
use specs::world::Index;
use hibitset::BitSetLike;
use shrev::EventChannel;
use shred::{Resources, SetupHandler};

pub enum HierarchyEvent {
    Modified(Entity),
    Removed(Entity),
}

pub struct Hierarchy<P> {
    sorted: Vec<Entity>,
    entities: HashMap<Index, usize>,
    children: HashMap<Entity, Vec<Entity>>,
    current_parent: HashMap<Entity, Entity>,
    changed: EventChannel<HierarchyEvent>,

    modified_id: ReaderId<ModifiedFlag>,
    inserted_id: ReaderId<InsertedFlag>,
    removed_id: ReaderId<RemovedFlag>,

    modified: BitSet,
    inserted: BitSet,
    removed: BitSet,

    scratch_set: HashSet<Entity>,

    _phantom: PhantomData<P>,
}

impl<P> Hierarchy<P> {
    /// Create a new hierarchy object.
    pub fn new(
        modified_id: ReaderId<ModifiedFlag>,
        inserted_id: ReaderId<InsertedFlag>,
        removed_id: ReaderId<RemovedFlag>,
    ) -> Self
    where
        P: Component,
        P::Storage: Tracked,
    {
        Hierarchy {
            sorted: Vec::new(),
            entities: HashMap::new(),
            current_parent: HashMap::new(),
            children: HashMap::new(),
            changed: EventChannel::new(),

            modified_id,
            inserted_id,
            removed_id,

            modified: BitSet::new(),
            inserted: BitSet::new(),
            removed: BitSet::new(),

            scratch_set: HashSet::default(),

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
        let ParentData {
            entities, parents, ..
        } = data;

        // Maintain tracking
        self.modified.clear();
        self.inserted.clear();
        self.removed.clear();

        parents.populate_modified(&mut self.modified_id, &mut self.modified);
        parents.populate_inserted(&mut self.inserted_id, &mut self.inserted);
        parents.populate_removed(&mut self.removed_id, &mut self.removed);

        // process removed parent components
        self.scratch_set.clear();
        for id in (&self.removed).iter() {
            if let Some(index) = self.entities.get(&id) {
                self.scratch_set.insert(self.sorted[*index]);
            }
        }

        // do removal
        if !self.scratch_set.is_empty() {
            let mut i = 0;
            let mut min_index = std::usize::MAX;
            while i < self.sorted.len() {
                let entity = self.sorted[i];
                let remove = self.scratch_set.contains(&entity)
                    || self.current_parent
                        .get(&entity)
                        .map(|parent_entity| self.scratch_set.contains(&parent_entity))
                        .unwrap_or(false);
                if remove {
                    if i < min_index {
                        min_index = i;
                    }
                    self.scratch_set.insert(entity);
                    self.sorted.remove(i);
                    if let Some(children) = self.current_parent
                        .get(&entity)
                        .cloned()
                        .and_then(|parent_entity| self.children.get_mut(&parent_entity))
                    {
                        if let Some(pos) = children.iter().position(|e| *e == entity) {
                            children.swap_remove(pos);
                        }
                    }
                    self.current_parent.remove(&entity);
                    self.children.remove(&entity);
                    self.entities.remove(&entity.id());
                } else {
                    i += 1;
                }
            }
            for i in min_index..self.sorted.len() {
                self.entities.insert(self.sorted[i].id(), i);
            }
            for entity in &self.scratch_set {
                self.changed.single_write(HierarchyEvent::Removed(*entity));
            }
        }

        // insert new components in hierarchy
        self.scratch_set.clear();
        for (entity, _, parent) in (&*entities, &self.inserted, &parents).join() {
            let parent_entity = parent.parent_entity();
            // if we insert a parent component on an entity that have children, we need to make
            // sure the parent is inserted before the children in the sorted list
            let insert_index = self.children
                .get(&entity)
                .and_then(|children| {
                    children
                        .iter()
                        .map(|child_entity| self.entities.get(&child_entity.id()).unwrap())
                        .min()
                        .cloned()
                })
                .unwrap_or(self.sorted.len());
            self.entities.insert(entity.id(), insert_index);
            if insert_index >= self.sorted.len() {
                self.sorted.push(entity);
            } else {
                self.sorted.insert(insert_index, entity);
                for i in insert_index..self.sorted.len() {
                    self.entities.insert(self.sorted[i].id(), i);
                }
            }

            let children = self.children
                .entry(parent_entity)
                .or_insert_with(Vec::default);
            children.push(entity);

            self.current_parent.insert(entity, parent_entity);
            self.scratch_set.insert(entity);
        }

        for (entity, _, parent) in (&*entities, &self.modified.clone(), &parents).join() {
            let parent_entity = parent.parent_entity();
            // if theres an old parent
            if let Some(old_parent) = self.current_parent.get(&entity).cloned() {
                // if the parent entity was not changed, ignore event
                if old_parent == parent_entity {
                    continue;
                }
                // remove entity from old parents children
                if let Some(children) = self.children.get_mut(&old_parent) {
                    if let Some(pos) = children.iter().position(|e| *e == entity) {
                        children.remove(pos);
                    }
                }
            }

            // insert in new parents children
            self.children
                .entry(parent_entity)
                .or_insert_with(Vec::default)
                .push(entity);

            // move entity in sorted if needed
            let entity_index = self.entities.get(&entity.id()).cloned().unwrap();
            if let Some(parent_index) = self.entities.get(&parent_entity.id()).cloned() {
                let mut offset = 0;
                let mut process_index = parent_index;
                while process_index > entity_index {
                    let move_entity = self.sorted.remove(process_index);
                    self.sorted.insert(entity_index, move_entity);
                    offset += 1;
                    process_index = self.current_parent
                        .get(&move_entity)
                        .and_then(|p_entity| self.entities.get(&p_entity.id()))
                        .map(|p_index| p_index + offset)
                        .unwrap_or(0);
                }

                // fix indexes
                if parent_index > entity_index {
                    for i in entity_index..parent_index {
                        self.entities.insert(self.sorted[i].id(), i);
                    }
                }
            }

            self.current_parent.insert(entity, parent_entity);
            self.scratch_set.insert(entity);
        }

        if !self.scratch_set.is_empty() {
            for i in 0..self.sorted.len() {
                let entity = self.sorted[i];
                let notify = self.scratch_set.contains(&entity)
                    || self.current_parent
                        .get(&entity)
                        .map(|parent_entity| self.scratch_set.contains(&parent_entity))
                        .unwrap_or(false);
                if notify {
                    self.scratch_set.insert(entity);
                    self.changed.single_write(HierarchyEvent::Modified(entity));
                }
            }
        }
    }
}

pub trait Parent {
    fn parent_entity(&self) -> Entity;
}

pub struct HierarchySetupHandler<P> {
    _m: PhantomData<P>,
}

impl<P> SetupHandler<Hierarchy<P>> for HierarchySetupHandler<P>
where
    P: Component + Send + Sync + 'static,
    P::Storage: Tracked,
{
    fn setup(res: &mut Resources) {
        if !res.has_value::<Hierarchy<P>>() {
            let hierarchy = {
                let mut storage: WriteStorage<P> = SystemData::fetch(&res);
                Hierarchy::<P>::new(
                    storage.track_modified(),
                    storage.track_inserted(),
                    storage.track_removed(),
                )
            };
            res.insert(hierarchy);
        }
    }
}

#[derive(SystemData)]
pub struct ParentData<'a, P>
where
    P: Component + Parent,
    P::Storage: Tracked,
{
    entities: Entities<'a>,
    parents: ReadStorage<'a, P>,
}

pub struct HierarchySystem<P> {
    m: PhantomData<P>,
}

impl<P> HierarchySystem<P> {
    pub fn new() -> Self {
        HierarchySystem { m: PhantomData }
    }
}

impl<'a, P> System<'a> for HierarchySystem<P>
where
    P: Component + Parent + Send + Sync + 'static,
    P::Storage: Tracked,
{
    type SystemData = (
        ParentData<'a, P>,
        Write<'a, Hierarchy<P>, HierarchySetupHandler<P>>,
    );

    fn run(&mut self, (data, mut hierarchy): Self::SystemData) {
        hierarchy.maintain(data);
    }
}
