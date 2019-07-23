# `specs-hierarchy`

[![Build Status][bi]][bl] [![Crates.io][ci]][cl] [![Gitter][gi]][gl] ![MIT/Apache][li] [![Docs.rs][di]][dl] ![LoC][lo]

[bi]: https://travis-ci.org/rustgd/specs-hierarchy.svg?branch=master
[bl]: https://travis-ci.org/rustgd/specs-hierarchy

[ci]: https://img.shields.io/crates/v/specs-hierarchy.svg
[cl]: https://crates.io/crates/specs-hierarchy/

[li]: https://img.shields.io/crates/l/specs-hierarchy.svg

[di]: https://docs.rs/specs-hierarchy/badge.svg
[dl]: https://docs.rs/specs-hierarchy/

[gi]: https://badges.gitter.im/slide-rs/specs.svg
[gl]: https://gitter.im/slide-rs/specs

[lo]: https://tokei.rs/b1/github/rustgd/specs-hierarchy?category=code

Scene graph type hierarchy abstraction for use with [`specs`].

Builds up a `Hierarchy` resource, by querying a user supplied `Parent` component.
Requires the component to be `Tracked`.

Will send modification events on an internal `EventChannel`. Note that `Removed` events
does not mean the `Parent` component was removed from the component storage, just that the
`Entity` will no longer be considered to be a part of the `Hierarchy`. This is because the user
may wish to either remove only the component, the complete `Entity`, or something completely
different. When an `Entity` that is a parent gets removed from the hierarchy, the full tree of
children below it will also be removed from the hierarchy.

[`specs`]: https://github.com/slide-rs/specs

## Usage

```toml
# Cargo.toml
[dependencies]
specs-hierarchy = "0.5.0"
```

## Example

```rust
use specs::prelude::{Component, DenseVecStorage, Entity, FlaggedStorage};
use specs_hierarchy::{Hierarchy, Parent as HParent};

/// Component for defining a parent entity.
///
/// The entity with this component *has* a parent, rather than *is* a parent.
#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Parent {
    /// The parent entity
    pub entity: Entity,
}

impl Component for Parent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl HParent for Parent {
    fn parent_entity(&self) -> Entity {
        self.entity
    }
}
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

We are a community project that welcomes contribution from anyone. If you're interested in helping out, you can contact
us either through GitHub, or via [`gitter`](https://gitter.im/slide-rs/specs).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
