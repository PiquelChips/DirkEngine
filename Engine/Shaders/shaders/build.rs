use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SpirvBuilder::new(shader_crate, target)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
