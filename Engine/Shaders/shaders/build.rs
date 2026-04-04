use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SpirvBuilder::new("../main", "spirv-unknown-vulkan1.3")
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
