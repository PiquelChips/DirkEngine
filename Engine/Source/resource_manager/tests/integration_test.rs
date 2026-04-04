use resource_manager::ResourceManager;

// -- happy-path --

#[test]
fn load_test_model_returns_ok() {
    let result = ResourceManager::load_model("test");
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
}

#[test]
fn load_test_model_mesh_count() {
    let model = ResourceManager::load_model("test").unwrap();
    assert_eq!(
        model.meshes().len(),
        1,
        "test asset should have exactly 1 mesh"
    );
}

#[test]
fn load_test_model_has_triangle_indices() {
    let model = ResourceManager::load_model("test").unwrap();
    let primitive = &model.meshes()[0].primitives()[0];
    // Our test asset is a single triangle → 3 indices
    assert_eq!(primitive.indices().len(), 3);
}

#[test]
fn load_test_model_positions_are_3d() {
    let model = ResourceManager::load_model("test").unwrap();
    let primitive = &model.meshes()[0].primitives()[0];
    for pos in primitive.positions() {
        assert_eq!(pos.len(), 3, "each position must be [x, y, z]");
    }
}

#[test]
fn load_test_model_no_material_on_untextured_mesh() {
    let model = ResourceManager::load_model("test").unwrap();
    let primitive = &model.meshes()[0].primitives()[0];
    assert!(
        primitive.material().is_none(),
        "untextured test asset should have no material index"
    );
}

// -- error path --

#[test]
fn load_nonexistent_model_returns_err() {
    let result = ResourceManager::load_model("this_model_does_not_exist_xyzzy");
    assert!(
        result.is_err(),
        "loading a missing model should return Err, not panic"
    );
}
