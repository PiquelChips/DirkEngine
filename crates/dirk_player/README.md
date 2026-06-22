# `dirk_player`

This crate manages player identity and lifecycle.

Register `PlayerPlugin` with an `EngineBuilder` to install player management
and movement ECS systems. The plugin publishes a `PlayerRegistry` resource that
other subsystem factories can clone.

`dirk_player` does not own ECS entities or camera state directly. Instead, it
provides:

- `PlayerId`, a component that links a game entity to a player
- `PlayerRegistry`, which allocates and tracks live players
- `PlayerSpawned` and `PlayerDespawned` events for lifecycle integration
- `PlayerInputSender`, which sends normalized targeted input to a player

In non-editor builds, it also provides `PlayerPresentationAssignments`, which
maps platform windows to players for presentation and input routing.

Game code is responsible for responding to those events by spawning or
despawning ECS entities with the corresponding `PlayerId` component.
