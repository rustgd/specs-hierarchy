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
    world.register::<Parent>();
    let mut system = HierarchySystem::<Parent>::new();
    system.setup(&mut world.res);

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
        let mut parents = world.write::<Parent>();
        parents.insert(e1, Parent { entity: e5 });
        parents.insert(e3, Parent { entity: e1 });
        parents.insert(e4, Parent { entity: e5 });
        parents.insert(e5, Parent { entity: e2 });
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(system, "hierarchy_system", &[])
        .build();

    dispatcher.dispatch(&mut world.res);

    {
        let parents = world.read::<Parent>();
        for entity in world.read_resource::<Hierarchy<Parent>>().all() {
            let formatted = parents
                .get(*entity)
                .map(|parent| format!("{:?}", parent.entity))
                .unwrap_or(format!("None"));
            println!("{:?}: {}", entity, formatted);
        }
    }
}
