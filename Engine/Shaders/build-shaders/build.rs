use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = "spirv-unknown-vulkan1.3";
    SpirvBuilder::new("../main", target)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
