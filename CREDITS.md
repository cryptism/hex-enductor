# Credits

Third-party assets bundled in this repository, beyond the MIT-licensed code
covered by [LICENSE](LICENSE).

## Link icons

`crates/map-core/src/link_icons.rs` bundles icons from
[game-icons.net](https://game-icons.net), licensed
[CC BY 3.0](https://creativecommons.org/licenses/by/3.0/):

- [Delapouite](https://delapouite.com)
- [Lorc](https://lorcblog.blogspot.com)

Regenerate that file with `cargo run -p hexen-cli -- sync-link-icons` (add
new icons to `ICON_SOURCES` in `apps/hexen-cli/src/icons.rs` first).
