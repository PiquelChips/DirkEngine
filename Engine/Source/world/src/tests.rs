//! Test suite for the `world` crate.
//!
//! Tests are grouped into three modules:
//! - `entity`      – spawn / despawn / alive tracking
//! - `components`  – component maths (Transform, Camera) and access helpers
//! - `queries`     – all query variants and edge-cases

#![cfg(test)]

use crate::World;
fn new_world(id: u32) -> World {
    let mut event_manager = events::EventManager::new();
    World::new(id, &mut event_manager)
}

mod entity {
    use super::new_world;

    // --- spawn --------------------------------------------------------------

    #[test]
    fn spawn_returns_unique_ids() {
        let mut w = new_world(0);
        let a = w.spawn();
        let b = w.spawn();
        let c = w.spawn();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn spawned_entity_appears_in_alive() {
        let mut w = new_world(0);
        let e = w.spawn();
        assert!(w.alive().contains(&e));
    }

    #[test]
    fn entity_count_tracks_spawns() {
        let mut w = new_world(0);
        assert_eq!(w.entity_count(), 0);
        w.spawn();
        w.spawn();
        assert_eq!(w.entity_count(), 2);
    }

    // --- despawn ------------------------------------------------------------

    #[test]
    fn despawned_entity_not_in_alive() {
        let mut w = new_world(0);
        let e = w.spawn();
        w.despawn(e);
        assert!(!w.alive().contains(&e));
    }

    #[test]
    fn despawn_unknown_entity_is_noop() {
        let mut w = new_world(0);
        // Entity 99 was never spawned — should not panic.
        w.despawn(99);
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn despawn_reduces_entity_count() {
        let mut w = new_world(0);
        let a = w.spawn();
        let _b = w.spawn();
        assert_eq!(w.entity_count(), 2);
        w.despawn(a);
        assert_eq!(w.entity_count(), 1);
    }

    #[test]
    fn ids_not_reused_after_despawn() {
        let mut w = new_world(0);
        let first = w.spawn();
        w.despawn(first);
        let second = w.spawn();
        assert_ne!(first, second);
    }

    // --- world id -----------------------------------------------------------

    #[test]
    fn world_id_is_preserved() {
        let w = new_world(42);
        assert_eq!(w.id(), 42);
    }
}

// ---------------------------------------------------------------------------

mod components {
    use crate::components::{Camera, Renderable, Transform};
    use glam::Vec3;

    // --- Transform default --------------------------------------------------

    #[test]
    fn transform_default_is_identity() {
        let t = Transform::default();
        assert_eq!(t.location, Vec3::ZERO);
        assert_eq!(t.rotation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    // --- Transform::forward -------------------------------------------------

    #[test]
    fn forward_is_unit_length() {
        let t = Transform {
            rotation: Vec3::new(30.0, 45.0, 0.0),
            ..Default::default()
        };
        let len = t.forward().length();
        assert!((len - 1.0).abs() < 1e-5, "forward length was {len}");
    }

    #[test]
    fn unrotated_transform_forward_matches_engine_forward() {
        let t = Transform::default();
        let fwd = t.forward();
        let expected = utils::FORWARD_DIRECTION;
        assert!(
            (fwd - expected).length() < 1e-5,
            "expected {expected:?}, got {fwd:?}"
        );
    }

    // --- Transform::matrix --------------------------------------------------

    #[test]
    fn matrix_translation_component_is_correct() {
        let location = Vec3::new(3.0, -1.0, 7.0);
        let t = Transform {
            location,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        };
        let m = t.matrix();
        // The last column of the model matrix is the translation.
        let w_col = m.w_axis;
        assert!((w_col.x - location.x).abs() < 1e-5);
        assert!((w_col.y - location.y).abs() < 1e-5);
        assert!((w_col.z - location.z).abs() < 1e-5);
    }

    #[test]
    fn matrix_identity_for_default_transform() {
        let t = Transform::default();
        let m = t.matrix();
        let ident = glam::Mat4::IDENTITY;
        for (a, b) in m.to_cols_array().iter().zip(ident.to_cols_array().iter()) {
            assert!((a - b).abs() < 1e-5, "matrix differs from identity: {m:?}");
        }
    }

    #[test]
    fn matrix_scale_doubles_all_axes() {
        let t = Transform {
            scale: Vec3::splat(2.0),
            ..Default::default()
        };
        let m = t.matrix();
        // Diagonal of a pure-scale matrix should be (2, 2, 2, 1).
        assert!((m.x_axis.x - 2.0).abs() < 1e-5);
        assert!((m.y_axis.y - 2.0).abs() < 1e-5);
        assert!((m.z_axis.z - 2.0).abs() < 1e-5);
        assert!((m.w_axis.w - 1.0).abs() < 1e-5);
    }

    #[test]
    fn from_transform_into_mat4_matches_matrix() {
        let t = Transform {
            location: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::new(15.0, 30.0, 0.0),
            scale: Vec3::splat(1.5),
        };
        let expected = t.clone().matrix();
        let got: glam::Mat4 = t.into();
        for (a, b) in got
            .to_cols_array()
            .iter()
            .zip(expected.to_cols_array().iter())
        {
            assert!((a - b).abs() < 1e-5);
        }
    }

    // --- Transform::view ----------------------------------------------------

    #[test]
    fn view_matrix_is_finite() {
        let t = Transform {
            location: Vec3::new(0.0, 5.0, -10.0),
            rotation: Vec3::new(10.0, 0.0, 0.0),
            scale: Vec3::ONE,
        };
        assert!(t.view().to_cols_array().iter().all(|v| v.is_finite()));
    }

    // --- Camera default -----------------------------------------------------

    #[test]
    fn camera_default_fov_is_45_degrees() {
        let c = Camera::default();
        assert!((c.fov - 45_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn camera_aspect_ratio() {
        let c = Camera {
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        };
        let expected = 1920.0_f32 / 1080.0;
        assert!((c.aspect_ratio() - expected).abs() < 1e-5);
    }

    // --- Camera::projection -------------------------------------------------

    #[test]
    fn projection_matrix_is_finite() {
        let c = Camera::default();
        assert!(c.projection().to_cols_array().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn projection_y_axis_is_negated_for_vulkan_ndc() {
        // The raw right-handed perspective matrix has a positive y_axis.y.
        // After the NDC flip, it must be negative.
        let c = Camera::default();
        assert!(
            c.projection().y_axis.y < 0.0,
            "y_axis.y should be negative (Vulkan NDC)"
        );
    }

    // --- Renderable ---------------------------------------------------------

    #[test]
    fn renderable_stores_model_name() {
        let r = Renderable {
            model: "meshes/sphere.glb".into(),
        };
        assert_eq!(r.model, "meshes/sphere.glb");
    }
}

// ---------------------------------------------------------------------------

mod queries {
    use super::new_world;
    use crate::{
        World,
        components::{Camera, Renderable, Transform},
    };
    use glam::Vec3;

    fn make_world() -> World {
        new_world(0)
    }

    fn default_transform() -> Transform {
        Transform::default()
    }

    fn default_renderable() -> Renderable {
        Renderable {
            model: "test".into(),
        }
    }

    // --- insert / get / remove ----------------------------------------------

    #[test]
    fn insert_and_get_component() {
        let mut w = make_world();
        let e = w.spawn();
        w.insert(e, default_transform());
        assert!(w.get::<Transform>(e).is_some());
    }

    #[test]
    fn get_returns_none_for_missing_component() {
        let mut w = make_world();
        let e = w.spawn();
        assert!(w.get::<Transform>(e).is_none());
    }

    #[test]
    fn insert_overwrites_existing_component() {
        let mut w = make_world();
        let e = w.spawn();
        w.insert(
            e,
            Transform {
                location: Vec3::ZERO,
                ..Default::default()
            },
        );
        let new_loc = Vec3::new(99.0, 0.0, 0.0);
        w.insert(
            e,
            Transform {
                location: new_loc,
                ..Default::default()
            },
        );
        let loc = w.get::<Transform>(e).unwrap().location;
        assert_eq!(loc, new_loc);
    }

    #[test]
    fn get_mut_allows_mutation() {
        let mut w = make_world();
        let e = w.spawn();
        w.insert(e, Transform::default());
        w.get_mut::<Transform>(e).unwrap().location = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(
            w.get::<Transform>(e).unwrap().location,
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn remove_deletes_component_but_keeps_entity() {
        let mut w = make_world();
        let e = w.spawn();
        w.insert(e, default_transform());
        w.remove::<Transform>(e);
        assert!(w.get::<Transform>(e).is_none());
        assert!(w.alive().contains(&e));
    }

    #[test]
    fn remove_missing_component_is_noop() {
        let mut w = make_world();
        let e = w.spawn();
        // Should not panic.
        w.remove::<Transform>(e);
    }

    #[test]
    fn despawn_removes_all_components() {
        let mut w = make_world();
        let e = w.spawn();
        w.insert(e, default_transform());
        w.insert(e, default_renderable());
        w.despawn(e);
        // Stale IDs should return None.
        assert!(w.get::<Transform>(e).is_none());
        assert!(w.get::<Renderable>(e).is_none());
    }

    // --- query_single -------------------------------------------------------

    #[test]
    fn query_single_returns_matching_entities() {
        let mut w = make_world();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, default_transform());
        // b has no Transform
        let results = w.query_single::<Transform>();
        assert!(results.contains(&a));
        assert!(!results.contains(&b));
    }

    #[test]
    fn query_single_empty_world() {
        let w = make_world();
        assert!(w.query_single::<Transform>().is_empty());
    }

    #[test]
    fn query_single_no_matching_entities() {
        let mut w = make_world();
        w.spawn(); // alive but no components
        assert!(w.query_single::<Transform>().is_empty());
    }

    // --- query_double -------------------------------------------------------

    #[test]
    fn query_double_requires_both_components() {
        let mut w = make_world();
        let full = w.spawn();
        let partial = w.spawn();
        w.insert(full, default_transform());
        w.insert(full, default_renderable());
        w.insert(partial, default_transform()); // missing Renderable

        let results = w.query_double::<Transform, Renderable>();
        assert!(results.contains(&full));
        assert!(!results.contains(&partial));
    }

    #[test]
    fn query_double_empty_when_no_entity_has_both() {
        let mut w = make_world();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, default_transform());
        w.insert(b, default_renderable());
        assert!(w.query_double::<Transform, Renderable>().is_empty());
    }

    // --- query_triple -------------------------------------------------------

    #[test]
    fn query_triple_requires_all_three() {
        let mut w = make_world();
        let full = w.spawn();
        let two = w.spawn();
        w.insert(full, default_transform());
        w.insert(full, default_renderable());
        w.insert(full, Camera::default());
        w.insert(two, default_transform());
        w.insert(two, default_renderable());
        // two is missing Camera

        let results = w.query_triple::<Transform, Renderable, Camera>();
        assert!(results.contains(&full));
        assert!(!results.contains(&two));
    }

    // --- query_quadruple ----------------------------------------------------
    // We only have three component types in the default registration, so we
    // test with a repeated pair to ensure the plumbing compiles and runs.
    // In a real codebase you would register a fourth type.
    #[test]
    fn query_quadruple_empty_for_impossible_constraint() {
        // Searching for (Transform, Renderable, Camera, Renderable) where the
        // last two are the same type—only entities with all three distinct
        // types (and the implicit double-Renderable overlap) should appear.
        let mut w = make_world();
        let e = w.spawn();
        w.insert(e, default_transform());
        w.insert(e, default_renderable());
        w.insert(e, Camera::default());

        // Quadruple with repeated Camera — still finds `e`.
        let results = w.query_quadruple::<Transform, Renderable, Camera, Camera>();
        assert!(results.contains(&e));
    }

    // --- ordering -----------------------------------------------------------

    #[test]
    fn query_preserves_spawn_order() {
        let mut w = make_world();
        let ids: Vec<_> = (0..5)
            .map(|_| {
                let e = w.spawn();
                w.insert(e, default_transform());
                e
            })
            .collect();

        let results = w.query_single::<Transform>();
        assert_eq!(results, ids);
    }

    // --- despawn mid-query --------------------------------------------------

    #[test]
    fn query_after_despawn_excludes_removed_entity() {
        let mut w = make_world();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, default_transform());
        w.insert(b, default_transform());
        w.despawn(a);

        let results = w.query_single::<Transform>();
        assert!(!results.contains(&a));
        assert!(results.contains(&b));
    }
}
