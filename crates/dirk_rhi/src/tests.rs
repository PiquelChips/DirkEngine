use crate::{BufferUsages, Extent3d, ImageUsages, PipelineStages, SampleCount};

#[test]
fn usage_flags_compose_without_backend_values() {
    let usage = BufferUsages::COPY_DST | BufferUsages::VERTEX;

    assert!(usage.contains(BufferUsages::COPY_DST));
    assert!(usage.contains(BufferUsages::VERTEX));
    assert!(!usage.contains(BufferUsages::UNIFORM));
}

#[test]
fn image_usage_flags_preserve_all_requested_roles() {
    let usage = ImageUsages::SAMPLED | ImageUsages::COLOR_ATTACHMENT | ImageUsages::COPY_SRC;

    assert!(usage.contains(ImageUsages::SAMPLED | ImageUsages::COPY_SRC));
    assert!(usage.contains(ImageUsages::COLOR_ATTACHMENT));
}

#[test]
fn semantic_types_do_not_encode_backend_constants() {
    assert_eq!(Extent3d::new_2d(1920, 1080).depth, 1);
    assert_eq!(SampleCount::Four as u8, 4);
    assert!(PipelineStages::ALL.contains(PipelineStages::COLOR_OUTPUT));
}
