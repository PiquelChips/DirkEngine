#![cfg(test)]

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

use parking_lot::{Mutex, RwLock};

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TestResource(&'static str);

struct CountingPlugin {
    builds: Arc<AtomicUsize>,
}

impl EnginePlugin for CountingPlugin {
    fn name(&self) -> &'static str {
        "counting"
    }

    fn build(&self, _builder: &mut EngineBuilder) -> anyhow::Result<()> {
        self.builds.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct SharedDependencyPlugin {
    builds: Arc<AtomicUsize>,
}

impl EnginePlugin for SharedDependencyPlugin {
    fn name(&self) -> &'static str {
        "shared-dependency"
    }

    fn build(&self, _builder: &mut EngineBuilder) -> anyhow::Result<()> {
        self.builds.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct DependentPlugin {
    dependency_builds: Arc<AtomicUsize>,
}

impl EnginePlugin for DependentPlugin {
    fn name(&self) -> &'static str {
        "dependent"
    }

    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()> {
        builder.with_plugin(SharedDependencyPlugin {
            builds: Arc::clone(&self.dependency_builds),
        })?;
        Ok(())
    }
}

struct OtherDependentPlugin {
    dependency_builds: Arc<AtomicUsize>,
}

impl EnginePlugin for OtherDependentPlugin {
    fn name(&self) -> &'static str {
        "other-dependent"
    }

    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()> {
        builder.with_plugin(SharedDependencyPlugin {
            builds: Arc::clone(&self.dependency_builds),
        })?;
        Ok(())
    }
}

struct PublishingSubsystem {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Subsystem for PublishingSubsystem {
    fn name(&self) -> &'static str {
        "publishing"
    }

    fn start(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        self.events.lock().push("publishing-start");
        Ok(())
    }

    fn tick(
        &mut self,
        _delta_time: f64,
        _handle: &EngineHandle,
        _universe: &mut Universe,
    ) -> anyhow::Result<()> {
        self.events.lock().push("publishing-tick");
        Ok(())
    }

    fn shutdown(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        self.events.lock().push("publishing-shutdown");
        Ok(())
    }
}

struct ReadingSubsystem {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Subsystem for ReadingSubsystem {
    fn name(&self) -> &'static str {
        "reading"
    }

    fn start(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        self.events.lock().push("reading-start");
        Ok(())
    }

    fn tick(
        &mut self,
        _delta_time: f64,
        handle: &EngineHandle,
        _universe: &mut Universe,
    ) -> anyhow::Result<()> {
        self.events.lock().push("reading-tick");
        handle.exit();
        Ok(())
    }

    fn shutdown(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        self.events.lock().push("reading-shutdown");
        Ok(())
    }
}

fn build_context() -> EngineBuildContext {
    let workers = WorkerPool::new("dirk-engine-test");
    let events = EventManager::new(workers.clone());
    let (commands, _command_receiver) = mpsc::channel();

    EngineBuildContext::new(EngineHandle {
        state: Arc::new(EngineState::new()),
        events,
        workers,
        commands,
        resources: Arc::new(RwLock::new(HashMap::new())),
    })
}

#[test]
fn registering_same_plugin_twice_only_builds_it_once() -> Result<()> {
    let builds = Arc::new(AtomicUsize::new(0));
    let mut builder = EngineBuilder::new();

    builder.with_plugin(CountingPlugin {
        builds: Arc::clone(&builds),
    })?;
    builder.with_plugin(CountingPlugin {
        builds: Arc::clone(&builds),
    })?;

    assert_eq!(builds.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn plugin_can_register_another_plugin_as_a_dependency() -> Result<()> {
    let dependency_builds = Arc::new(AtomicUsize::new(0));
    let mut builder = EngineBuilder::new();

    builder.with_plugin(DependentPlugin {
        dependency_builds: Arc::clone(&dependency_builds),
    })?;

    assert_eq!(dependency_builds.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn shared_dependency_is_registered_once_for_multiple_dependents() -> Result<()> {
    let dependency_builds = Arc::new(AtomicUsize::new(0));
    let mut builder = EngineBuilder::new();

    builder.with_plugin(DependentPlugin {
        dependency_builds: Arc::clone(&dependency_builds),
    })?;
    builder.with_plugin(OtherDependentPlugin {
        dependency_builds: Arc::clone(&dependency_builds),
    })?;

    assert_eq!(dependency_builds.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn resource_lookup_reports_missing_resources() {
    let context = build_context();

    let result = context.resource::<TestResource>();

    assert!(matches!(result, Err(Error::ResourceMissing { .. })));
}

#[test]
fn duplicate_resource_registration_reports_error() -> Result<()> {
    let mut context = build_context();

    context.add_resource(TestResource("first"))?;
    let result = context.add_resource(TestResource("second"));

    assert!(matches!(
        result,
        Err(Error::ResourceAlreadyRegistered { .. })
    ));
    Ok(())
}

#[test]
fn subsystem_factories_publish_and_read_resources_and_lifecycle_still_runs() -> Result<()> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut builder = EngineBuilder::new();

    {
        let events = Arc::clone(&events);
        builder.add_subsystem(move |ctx| {
            events.lock().push("publishing-factory");
            ctx.add_resource(TestResource("resource"))?;
            Ok(PublishingSubsystem { events })
        });
    }

    {
        let events = Arc::clone(&events);
        builder.add_subsystem(move |ctx| {
            let resource = ctx.resource::<TestResource>()?;
            assert_eq!(resource, TestResource("resource"));
            events.lock().push("reading-factory");
            Ok(ReadingSubsystem { events })
        });
    }

    let mut engine = builder.build()?;

    assert_eq!(&*events.lock(), &["publishing-factory", "reading-factory"]);

    let status = engine.tick()?;
    assert_eq!(status, EngineStatus::ExitRequested);
    engine.shutdown()?;

    assert_eq!(
        &*events.lock(),
        &[
            "publishing-factory",
            "reading-factory",
            "publishing-start",
            "reading-start",
            "publishing-tick",
            "reading-tick",
            "reading-shutdown",
            "publishing-shutdown",
        ]
    );

    Ok(())
}
