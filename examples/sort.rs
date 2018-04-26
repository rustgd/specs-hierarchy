extern crate specs;
extern crate specs_hierarchy;

use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchySystem};

struct Parent {
    entity: Entity,
}

impl Component for Parent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl specs_hierarchy::Parent for Parent {
    fn parent_entity(&self) -> Entity {
        self.entity
    }
}

fn main() {
    let mut world = World::new();
    let mut dispatcher = DispatcherBuilder::new()
        .with(HierarchySystem::<Parent>::new(), "hierarchy_system", &[])
        .build();
    dispatcher.setup(&mut world.res);

    let _e0 = world.create_entity().build();
    let e1 = world.create_entity().build();
    let e2 = world.create_entity().build();
    let e3 = world.create_entity().build();
    let e4 = world.create_entity().build();
    let e5 = world.create_entity().build();
    let _e6 = world.create_entity().build();
    let _e7 = world.create_entity().build();
    let _e8 = world.create_entity().build();
    let _e9 = world.create_entity().build();

    {
        let mut parents = world.write_storage::<Parent>();
        parents.insert(e1, Parent { entity: e5 });
        parents.insert(e3, Parent { entity: e1 });
        parents.insert(e4, Parent { entity: e5 });
        parents.insert(e5, Parent { entity: e2 });
    }

    dispatcher.dispatch(&mut world.res);

    {
        let parents = world.read_storage::<Parent>();
        for entity in world.read_resource::<Hierarchy<Parent>>().all() {
            let formatted = parents
                .get(*entity)
                .map(|parent| format!("{:?}", parent.entity))
                .unwrap_or(format!("None"));
            println!("{:?}: {}", entity, formatted);
        }
    }
}
