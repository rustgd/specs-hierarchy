/// # `specs-hierarchy`
///
/// Scene graph type hierarchy for use with Specs.
///
/// ## `Parent`
///
/// This crate uses a generic parameter `P` for the parent component. Its bound by the `Parent`
/// trait that only requires a getter for the `Entity` that's the parent.
///
/// ## Usage
///
/// ```rust
/// # extern crate specs;
/// # extern crate specs_hierarchy;
///
/// use specs::prelude::{Component, DenseVecStorage, Entity, FlaggedStorage};
/// use specs_hierarchy::{Hierarchy, Parent as HParent};
///
/// pub use specs_hierarchy::HierarchyEvent;
/// pub type ParentHierarchy = Hierarchy<Parent>;
///
/// /// Component for defining a parent entity.
/// ///
/// /// The entity with this component *has* a parent, rather than *is* a parent.
/// #[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
/// pub struct Parent {
///     /// The parent entity
///     pub entity: Entity,
/// }
///
/// impl Component for Parent {
///     type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
/// }
///
/// impl HParent for Parent {
///     fn parent_entity(&self) -> Entity {
///         self.entity
///     }
/// }
///
/// # fn main() {}
/// ```
///

extern crate hibitset;
extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate shrev;
extern crate specs;

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use hibitset::BitSetLike;
use shred::SetupHandler;
use shrev::EventChannel;
use specs::prelude::{BitSet, Component, Entities, Entity, InsertedFlag, Join, ModifiedFlag,
                     ReadStorage, ReaderId, RemovedFlag, Resources, System, SystemData, Tracked,
                     Write, WriteStorage};
use specs::world::Index;

/// Hierarchy events.
///
/// These are the events that are sent through the internal `EventChannel` in the `Hierarchy`
/// resource.
pub enum HierarchyEvent {
    /// `Entity` was either inserted or modified in the `Hierarchy`
    Modified(Entity),
    /// `Entity` was removed from the `Hierarchy`. Note that this does not mean the `Parent`
    /// component was removed from the component storage, just that the `Entity` will no longer be
    /// considered to be a part of the `Hierarchy`.
    Removed(Entity),
}

/// Scene graph type hierarchy.
///
/// Will use the given generic type `P` as the component type that provides parenting links. The
/// internal structure is kept in sync with the `Tracked` events for that component type.
///
/// Will send modification events on the internal `EventChannel`. Note that `Removed` events
/// do not indicate that the `Parent` component was removed from the component storage, just that
/// the `Entity` will no longer be considered to be a part of the `Hierarchy`. This is because the
/// user may wish to either remove only the component, or the complete Entity, or something
/// completely different. When an `Entity` that is a parent gets removed from the hierarchy, the
/// full tree of children below it will also be removed from the hierarchy.
///
/// Any cycles in the hierarchy will cause Undefined Behavior.
pub struct Hierarchy<P> {
    sorted: Vec<Entity>,
    entities: HashMap<Index, usize>,
    children: HashMap<Entity, Vec<Entity>>,
    current_parent: HashMap<Entity, Entity>,
    external_parents: HashSet<Entity>,
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
            external_parents: HashSet::new(),
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

    /// Get all entities that contain parents, in sorted order, where parents are guaranteed to
    /// be before their children.
    ///
    /// Note: This does not include entities that **are** parents.
    pub fn all(&self) -> &[Entity] {
        self.sorted.as_slice()
    }

    /// Get the children of a specific entity.
    pub fn children(&self, entity: Entity) -> &[Entity] {
        self.children.get(&entity).map(|vec| vec.as_slice()).unwrap_or(&[])
    }

    /// Get the parent of a specific entity
    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.current_parent.get(&entity).cloned()
    }

    /// Get a token for tracking the modification events from the hierarchy
    pub fn track(&mut self) -> ReaderId<HierarchyEvent> {
        self.changed.register_reader()
    }

    /// Get the `EventChannel` for the modification events for reading
    pub fn changed(&self) -> &EventChannel<HierarchyEvent> {
        &self.changed
    }

    /// Maintain the hierarchy, usually only called by `HierarchySystem`.
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
        // handle parents that have been removed which do not have a Parent themselves
        for entity in &self.external_parents {
            if !entities.is_alive(*entity) {
                self.scratch_set.insert(*entity);
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
                self.external_parents.remove(entity);
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

            {
                let children = self.children
                    .entry(parent_entity)
                    .or_insert_with(Vec::default);
                children.push(entity);
            }

            self.current_parent.insert(entity, parent_entity);
            self.scratch_set.insert(entity);
            if !self.current_parent.contains_key(&parent_entity) {
                self.external_parents.insert(parent_entity);
            }
            self.external_parents.remove(&entity);
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

            if !self.current_parent.contains_key(&parent_entity) {
                self.external_parents.insert(parent_entity);
            }
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

        self.scratch_set.clear();
        for entity in &self.external_parents {
            if !self.children.contains_key(entity) {
                self.scratch_set.insert(*entity);
            }
        }
        for entity in &self.scratch_set {
            self.external_parents.remove(entity);
        }
    }
}

/// Bound for the parent component of your crate. Your `Parent` component usually just contains the
/// `Entity` that's the parent you're linking to.
///
/// Note that the component should indicate that the `Entity` its added *has* a parent (the entity
/// stored in your component).
pub trait Parent {
    /// Retrieves the parent `Entity`.
    fn parent_entity(&self) -> Entity;
}

/// Specs `SetupHandler` for the `Hierarchy` resource.
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

/// Utility struct for the data needed by the `Hierarchy` maintain.
#[derive(SystemData)]
pub struct ParentData<'a, P>
where
    P: Component + Parent,
    P::Storage: Tracked,
{
    entities: Entities<'a>,
    parents: ReadStorage<'a, P>,
}

/// System for maintaining a `Hierarchy` resource.
///
/// ## Type parameters:
///
/// - `P`: Component type that provides `Parent` links for the maintained `Hierarchy`
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

#[cfg(test)]
mod tests {

    use super::{Hierarchy, HierarchyEvent, HierarchySystem, Parent as PParent};
    use specs::prelude::{Component, DenseVecStorage, Entity, FlaggedStorage, ReaderId, RunNow,
                         System, World};

    struct Parent {
        entity: Entity,
    }

    impl Component for Parent {
        type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
    }

    impl PParent for Parent {
        fn parent_entity(&self) -> Entity {
            self.entity
        }
    }

    fn delete_removals(world: &mut World, reader_id: &mut ReaderId<HierarchyEvent>) {
        let mut remove = vec![];
        for event in world
            .read_resource::<Hierarchy<Parent>>()
            .changed()
            .read(reader_id)
        {
            if let HierarchyEvent::Removed(entity) = *event {
                remove.push(entity);
            }
        }
        for entity in remove {
            if let Err(_) = world.delete_entity(entity) {
                println!("Failed removed entity");
            }
        }
    }

    #[test]
    fn parent_removed() {
        let mut world = World::new();
        world.register::<Parent>();
        let mut system = HierarchySystem::<Parent>::new();
        System::setup(&mut system, &mut world.res);
        let mut reader_id = world.write_resource::<Hierarchy<Parent>>().track();

        let e1 = world.create_entity().build();

        let e2 = world.create_entity().with(Parent { entity: e1 }).build();

        let e3 = world.create_entity().build();

        let e4 = world.create_entity().with(Parent { entity: e3 }).build();

        let e5 = world.create_entity().with(Parent { entity: e4 }).build();

        system.run_now(&mut world.res);
        delete_removals(&mut world, &mut reader_id);
        world.maintain();

        let _ = world.delete_entity(e1);
        system.run_now(&mut world.res);
        delete_removals(&mut world, &mut reader_id);
        world.maintain();

        assert_eq!(world.is_alive(e1), false);
        assert_eq!(world.is_alive(e2), false);

        let _ = world.delete_entity(e3);
        system.run_now(&mut world.res);
        delete_removals(&mut world, &mut reader_id);
        world.maintain();

        assert_eq!(world.is_alive(e3), false);
        assert_eq!(world.is_alive(e4), false);
        assert_eq!(world.is_alive(e5), false);

        assert_eq!(0, world.read_resource::<Hierarchy<Parent>>().all().len());
    }
}
