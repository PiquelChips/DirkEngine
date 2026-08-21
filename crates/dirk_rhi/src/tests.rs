use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    Buffer, BufferUsages, Error, Extent3d, Fence, ImageUsages, PipelineStages, Result, SampleCount,
    TimelineSemaphore,
};

#[derive(Clone, Default)]
struct TestBuffer(Arc<AtomicU64>);

impl Buffer for TestBuffer {
    fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        let size =
            u64::try_from(data.len()).map_err(|error| Error::Backend(anyhow::Error::new(error)))?;
        self.0.store(offset + size, Ordering::Release);
        Ok(())
    }
}

#[derive(Default)]
struct TestFence(AtomicBool);

impl Fence for TestFence {
    fn wait(&self, _timeout_ns: u64) -> Result<()> {
        self.0.store(true, Ordering::Release);
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        self.0.store(false, Ordering::Release);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct TestTimeline(Arc<AtomicU64>);

impl TimelineSemaphore for TestTimeline {
    fn wait(&self, value: u64, _timeout_ns: u64) -> Result<()> {
        self.0.fetch_max(value, Ordering::AcqRel);
        Ok(())
    }

    fn value(&self) -> Result<u64> {
        Ok(self.0.load(Ordering::Acquire))
    }
}

#[test]
fn usage_flags_compose_without_backend_values() {
    const DEFAULT: BufferUsages = BufferUsages::COPY_DST.union(BufferUsages::VERTEX);
    let usage = BufferUsages::COPY_DST | BufferUsages::VERTEX;

    assert_eq!(usage, DEFAULT);
    assert!(usage.contains(BufferUsages::COPY_DST));
    assert!(usage.contains(BufferUsages::VERTEX));
    assert!(!usage.contains(BufferUsages::UNIFORM));
    assert!(BufferUsages::NONE.is_empty());
    assert!(BufferUsages::from_bits(usage.bits()).is_some());
    assert!(BufferUsages::from_bits(u32::MAX).is_none());
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
    assert!(SampleCount::Four < SampleCount::Eight);
    assert_eq!(
        PipelineStages::ALL,
        PipelineStages::COPY
            | PipelineStages::VERTEX
            | PipelineStages::FRAGMENT
            | PipelineStages::COLOR_OUTPUT
            | PipelineStages::COMPUTE
    );
}

#[test]
fn resources_own_their_stateful_operations() -> Result<()> {
    let buffer = TestBuffer::default();
    buffer.write(8, &[1, 2, 3, 4])?;
    assert_eq!(buffer.0.load(Ordering::Acquire), 12);

    let fence = TestFence::default();
    fence.wait(u64::MAX)?;
    assert!(fence.0.load(Ordering::Acquire));
    fence.reset()?;
    assert!(!fence.0.load(Ordering::Acquire));

    let timeline = TestTimeline::default();
    timeline.wait(42, u64::MAX)?;
    assert_eq!(timeline.value()?, 42);
    Ok(())
}

#[test]
fn backend_errors_keep_anyhow_context() {
    let source = anyhow::anyhow!("native allocation failed").context("creating buffer");
    let error = Error::from(source);

    assert!(matches!(error, Error::Backend(_)));
    assert!(error.to_string().contains("creating buffer"));
}
