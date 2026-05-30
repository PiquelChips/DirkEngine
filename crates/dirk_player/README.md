# `dirk_player`

This crate manages player identity and lifecycle.

`dirk_player` does not own ECS entities or camera state directly. Instead, it
provides:

- `PlayerId`, a component that links a game entity to a player
- `PlayerManager`, which allocates and tracks live players
- `PlayerSpawned` and `PlayerDespawned` events for lifecycle integration

Game code is responsible for responding to those events by spawning or
despawning ECS entities with the corresponding `PlayerId` component.
