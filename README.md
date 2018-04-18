Scene graph type hierarchy abstraction for use with [`specs`].

Builds up a `Hierarchy` resource, by querying a user supplied `Parent` component. 
Requires the component to be `Tracked`.

/// Will send modification events on an internal `EventChannel`. Note that `Removed` events
/// does not mean the `Parent` component was removed from the component storage, just that the
/// `Entity` will no longer be considered to be a part of the `Hierarchy`. This is because the user
/// may wish to either remove only the component, the complete `Entity`, or something completely
/// different. When an `Entity` that is a parent gets removed from the hierarchy, the full tree of
/// children below it will also be removed from the hierarchy.
 
[`specs`]: https://github.com/slide-rs/specs

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

We are a community project that welcomes contribution from anyone. If you're interested in helping out, you can contact
us either through GitHub, or via [`gitter`](https://gitter.im/slide-rs/specs).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
