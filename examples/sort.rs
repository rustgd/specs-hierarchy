
extern crate specs;
extern crate specs_hierarchy;

use specs::*;
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
    world.add_resource::<Hierarchy<Parent, DenseVecStorage<Parent>>>(Hierarchy::new());

    let e0 = world.create_entity().build();
    let e1 = world.create_entity().build();
    let e2 = world.create_entity().build();
    let e3 = world.create_entity().build();
    let e4 = world.create_entity().build();
    let e5 = world.create_entity().build();
    let e6 = world.create_entity().build();
    let e7 = world.create_entity().build();
    let e8 = world.create_entity().build();
    let e9 = world.create_entity().build();

    {
        let mut parents = world.write::<Parent>();
        parents.insert(e1, Parent { entity: e5 });
        parents.insert(e3, Parent { entity: e1 });
        parents.insert(e4, Parent { entity: e5 });
        parents.insert(e5, Parent { entity: e2 });
    }

    let mut dispatcher = DispatcherBuilder::new()
        .add(HierarchySystem::<Parent, DenseVecStorage<Parent>>::new(), "hierarchy_system", &[])
        .build();

    dispatcher.dispatch(&mut world.res);

    {
        let parents = world.read::<Parent>();
        for entity in world.read_resource::<Hierarchy<Parent, DenseVecStorage<Parent>>>().all() {
            let formatted = match parents.get(*entity) {
                Some(parent) => format!("({}, {})", parent.entity.id(), parent.entity.gen().id()),
                None => format!("None"),
            };
            println!("({}, {}): {}", entity.id(), entity.gen().id(), formatted);
        }
    }
}
