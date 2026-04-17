use crate::AssetConfig;

impl AssetConfig {
    pub fn validate(&self) -> bool {
        // TODO: check that the specified asset type has the matching field set to some
        // TODO: validate specific subconfig (for ex: check .gltf actually exists)
        todo!("make sure asset configuration is valid")
    }
}
