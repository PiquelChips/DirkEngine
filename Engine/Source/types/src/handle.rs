use std::marker::PhantomData;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AssetHandle<T: AssetType> {
    pub id: u32,
    pub _marker: PhantomData<T>,
}

pub trait AssetType {}

macro_rules! declare_asset_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $name;
        impl AssetType for $name {}

        pastey::paste! {
            pub type [<$name Handle>] = AssetHandle<$name>;
        }
    };
}

declare_asset_type!(Mesh);
declare_asset_type!(Texture);
declare_asset_type!(Material);
declare_asset_type!(Model);
