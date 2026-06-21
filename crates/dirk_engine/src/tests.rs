#![cfg(test)]

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
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

struct SelfDependentPlugin;

impl EnginePlugin for SelfDependentPlugin {
    fn name(&self) -> &'static str {
        "self-dependent"
    }

    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()> {
        builder.with_plugin(SelfDependentPlugin)?;
        Ok(())
    }
}

struct StartFailingSubsystem;

impl Subsystem for StartFailingSubsystem {
    fn name(&self) -> &'static str {
        "start-failing"
    }

    fn start(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("start failed"))
    }
}

struct ShutdownFailingSubsystem;

impl Subsystem for ShutdownFailingSubsystem {
    fn name(&self) -> &'static str {
        "shutdown-failing"
    }

    fn shutdown(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("shutdown failed"))
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
        metadata: Arc::new(EngineMetadata::new(
            "test-app",
            dirk_utils::Version::ZERO,
            "test-engine",
            dirk_utils::Version::ZERO,
        )),
        state: Arc::new(EngineState::new()),
        events,
        workers,
        commands,
        resources: Arc::new(RwLock::new(HashMap::new())),
    })
}

pub(crate) fn engine_with_subsystems(subsystems: Vec<Box<dyn Subsystem>>) -> Engine {
    engine_with_subsystems_and_signals(
        subsystems,
        signal::OperatingSystemSignals::empty_for_tests(),
    )
}

fn engine_with_subsystems_and_signals(
    subsystems: Vec<Box<dyn Subsystem>>,
    signals: signal::OperatingSystemSignals,
) -> Engine {
    let workers = WorkerPool::new("dirk-engine-test");
    let events = EventManager::new(workers.clone());
    let state = Arc::new(EngineState::new());
    let (commands, command_receiver) = mpsc::channel();
    let resources = Arc::new(RwLock::new(HashMap::new()));
    let handle = EngineHandle {
        metadata: Arc::new(EngineMetadata::new(
            "test-app",
            dirk_utils::Version::ZERO,
            "test-engine",
            dirk_utils::Version::ZERO,
        )),
        state: Arc::clone(&state),
        events: events.clone(),
        workers,
        commands,
        resources,
    };

    Engine {
        logger: piquel_log::Logger::new(),
        universe: Universe::builder().build(),
        subsystems,
        #[cfg(feature = "editor")]
        editor: editor::EditorRuntime::empty_for_tests(),
        state,
        handle,
        command_receiver,
        signals,
        frame_dispatcher: events.register(),
        exiting_dispatcher: events.register(),
        last_tick: Instant::now(),
        started: false,
        shutdown: false,
    }
}

struct CountingTickSubsystem {
    ticks: Arc<AtomicUsize>,
}

impl Subsystem for CountingTickSubsystem {
    fn name(&self) -> &'static str {
        "counting-tick"
    }

    fn tick(
        &mut self,
        _delta_time: f64,
        _handle: &EngineHandle,
        _universe: &mut Universe,
    ) -> anyhow::Result<()> {
        self.ticks.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
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
fn cyclic_plugin_dependency_reports_error() {
    let mut builder = EngineBuilder::new();

    let result = builder.with_plugin(SelfDependentPlugin);

    match result {
        Err(Error::PluginBuildFailed {
            name: "self-dependent",
            source,
        }) => assert!(matches!(
            source.downcast_ref::<Error>(),
            Some(Error::PluginDependencyCycle {
                name: "self-dependent",
                ..
            })
        )),
        _ => panic!("expected self-dependent plugin build to fail with a cycle"),
    }
}

#[test]
fn subsystem_start_failure_uses_start_error_variant() {
    let mut engine = engine_with_subsystems(vec![Box::new(StartFailingSubsystem)]);

    let result = engine.start();

    assert!(matches!(
        result,
        Err(Error::SubsystemFailedStart {
            name: "start-failing",
            ..
        })
    ));
}

#[test]
fn subsystem_shutdown_failure_uses_shutdown_error_variant() {
    let mut engine = engine_with_subsystems(vec![Box::new(ShutdownFailingSubsystem)]);

    let result = engine.shutdown();

    assert!(matches!(
        result,
        Err(Error::SubsystemFailedShutdown {
            name: "shutdown-failing",
            ..
        })
    ));
}

#[test]
fn operating_system_signals_are_processed_before_subsystem_ticks() -> Result<()> {
    let ticks = Arc::new(AtomicUsize::new(0));
    let signals =
        signal::OperatingSystemSignals::with_signal_for_tests(signal_hook::consts::SIGINT)
            .map_err(Error::SubsystemFailedInit)?;
    let mut engine = engine_with_subsystems_and_signals(
        vec![Box::new(CountingTickSubsystem {
            ticks: Arc::clone(&ticks),
        })],
        signals,
    );

    let status = engine.tick()?;

    assert_eq!(status, EngineStatus::ExitRequested);
    assert_eq!(ticks.load(Ordering::Relaxed), 0);
    engine.shutdown()?;
    Ok(())
}

#[test]
fn metadata_is_available_on_engine_handle() {
    let context = build_context();
    let metadata = context.handle().metadata();

    assert_eq!(metadata.app_name(), "test-app");
    assert_eq!(metadata.app_version(), dirk_utils::Version::ZERO);
    assert_eq!(metadata.engine_name(), "test-engine");
    assert_eq!(metadata.engine_version(), dirk_utils::Version::ZERO);
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

#[cfg(feature = "editor")]
mod editor_tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use parking_lot::Mutex;

    use super::*;
    use crate::editor::{
        EditorMenuDescriptor, EditorRenderContext, EditorServices, EditorSubsystem,
        EditorTickContext, EditorWindowDescriptor, EditorWindowInfo,
    };

    struct FirstEditorSubsystem {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl EditorSubsystem for FirstEditorSubsystem {
        fn name(&self) -> &'static str {
            "first-editor"
        }

        fn start(
            &mut self,
            _context: &mut crate::editor::EditorStartContext<'_>,
        ) -> anyhow::Result<()> {
            self.events.lock().push("first-start");
            Ok(())
        }

        fn tick(&mut self, _context: &mut EditorTickContext<'_>) -> anyhow::Result<()> {
            self.events.lock().push("first-tick");
            Ok(())
        }

        fn shutdown(
            &mut self,
            _context: &mut crate::editor::EditorShutdownContext<'_>,
        ) -> anyhow::Result<()> {
            self.events.lock().push("first-shutdown");
            Ok(())
        }
    }

    struct SecondEditorSubsystem {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl EditorSubsystem for SecondEditorSubsystem {
        fn name(&self) -> &'static str {
            "second-editor"
        }

        fn start(
            &mut self,
            _context: &mut crate::editor::EditorStartContext<'_>,
        ) -> anyhow::Result<()> {
            self.events.lock().push("second-start");
            Ok(())
        }

        fn tick(&mut self, _context: &mut EditorTickContext<'_>) -> anyhow::Result<()> {
            self.events.lock().push("second-tick");
            Ok(())
        }

        fn shutdown(
            &mut self,
            _context: &mut crate::editor::EditorShutdownContext<'_>,
        ) -> anyhow::Result<()> {
            self.events.lock().push("second-shutdown");
            Ok(())
        }
    }

    struct DuplicateEditorSubsystem;

    impl EditorSubsystem for DuplicateEditorSubsystem {
        fn name(&self) -> &'static str {
            "duplicate-editor"
        }
    }

    struct StartFailingEditorSubsystem;

    impl EditorSubsystem for StartFailingEditorSubsystem {
        fn name(&self) -> &'static str {
            "start-failing-editor"
        }

        fn start(
            &mut self,
            _context: &mut crate::editor::EditorStartContext<'_>,
        ) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("editor start failed"))
        }
    }

    fn render_services(services: &EditorServices, universe: &Universe) -> anyhow::Result<()> {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..egui::RawInput::default()
        });

        let handle = build_context().handle().clone();
        let frame = EditorRenderContext::new(0.016, &handle, universe, services);
        let result = services.render_ui(&ctx, &frame);
        let _ = ctx.end_pass();
        result
    }

    fn descriptor(title: &str, default_open: bool) -> EditorWindowDescriptor {
        EditorWindowDescriptor {
            title: title.to_owned(),
            category: "Tests".to_owned(),
            default_open,
            show_in_list: true,
        }
    }

    #[test]
    fn editor_subsystem_registration_order_drives_lifecycle_order() -> Result<()> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut builder = EngineBuilder::new();

        {
            let events = Arc::clone(&events);
            builder.add_editor_subsystem(move |_ctx| Ok(FirstEditorSubsystem { events }));
        }
        {
            let events = Arc::clone(&events);
            builder.add_editor_subsystem(move |_ctx| Ok(SecondEditorSubsystem { events }));
        }

        let mut engine = builder.build()?;
        engine.start()?;
        engine.tick()?;
        engine.shutdown()?;

        assert_eq!(
            &*events.lock(),
            &[
                "first-start",
                "second-start",
                "first-tick",
                "second-tick",
                "second-shutdown",
                "first-shutdown",
            ]
        );
        Ok(())
    }

    #[test]
    fn duplicate_editor_subsystem_registration_is_skipped() -> Result<()> {
        let builds = Arc::new(AtomicUsize::new(0));
        let mut builder = EngineBuilder::new();

        {
            let builds = Arc::clone(&builds);
            builder.add_editor_subsystem(move |_ctx| {
                builds.fetch_add(1, Ordering::Relaxed);
                Ok(DuplicateEditorSubsystem)
            });
        }
        {
            let builds = Arc::clone(&builds);
            builder.add_editor_subsystem(move |_ctx| {
                builds.fetch_add(1, Ordering::Relaxed);
                Ok(DuplicateEditorSubsystem)
            });
        }

        let _engine = builder.build()?;

        assert_eq!(builds.load(Ordering::Relaxed), 1);
        Ok(())
    }

    #[test]
    fn editor_lifecycle_errors_include_subsystem_names() -> Result<()> {
        let mut builder = EngineBuilder::new();
        builder.add_editor_subsystem(|_ctx| Ok(StartFailingEditorSubsystem));
        let mut engine = builder.build()?;

        let result = engine.start();

        assert!(matches!(
            result,
            Err(Error::EditorSubsystemFailedStart {
                name: "start-failing-editor",
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn editor_services_are_published_as_an_engine_resource() -> Result<()> {
        let observed = Arc::new(Mutex::new(None));
        let observed_services = Arc::clone(&observed);
        let mut builder = EngineBuilder::new();

        builder.add_editor_subsystem(move |ctx| {
            *observed_services.lock() = Some(ctx.engine.resource::<EditorServices>()?);
            Ok(DuplicateEditorSubsystem)
        });

        let _engine = builder.build()?;

        assert!(observed.lock().is_some());
        Ok(())
    }

    #[test]
    fn window_ids_are_stable_and_increasing() {
        let services = EditorServices::new();

        let first = services.add_window_fn(descriptor("first", false), |_ui, _context| Ok(()));
        let second = services.add_window_fn(descriptor("second", false), |_ui, _context| Ok(()));

        assert_eq!(first.raw(), 0);
        assert_eq!(second.raw(), 1);
    }

    #[test]
    fn window_default_open_state_is_honored() {
        let services = EditorServices::new();

        let open = services.add_window_fn(descriptor("open", true), |_ui, _context| Ok(()));
        let closed = services.add_window_fn(descriptor("closed", false), |_ui, _context| Ok(()));

        assert_eq!(services.is_open(open), Some(true));
        assert_eq!(services.is_open(closed), Some(false));
    }

    #[test]
    fn closed_windows_do_not_render() -> anyhow::Result<()> {
        let services = EditorServices::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let callback_calls = Arc::clone(&calls);
        let id = services.add_window_fn(descriptor("window", true), move |_ui, _context| {
            callback_calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        services.set_open(id, false);

        let universe = Universe::builder().build();
        render_services(&services, &universe)?;

        assert_eq!(calls.load(Ordering::Relaxed), 0);
        Ok(())
    }

    #[test]
    fn open_windows_render_in_registration_order() -> anyhow::Result<()> {
        let services = EditorServices::new();
        let calls = Arc::new(Mutex::new(Vec::new()));

        for label in ["first", "second", "third"] {
            let calls = Arc::clone(&calls);
            services.add_window_fn(descriptor(label, true), move |_ui, _context| {
                calls.lock().push(label);
                Ok(())
            });
        }

        let universe = Universe::builder().build();
        render_services(&services, &universe)?;

        assert_eq!(*calls.lock(), vec!["first", "second", "third"]);
        Ok(())
    }

    #[test]
    fn window_render_errors_include_window_title() {
        let services = EditorServices::new();
        services.add_window_fn(descriptor("error", true), |_ui, _context| {
            Err(anyhow::anyhow!("window failed"))
        });

        let universe = Universe::builder().build();
        let err = render_services(&services, &universe).expect_err("render should fail");

        assert!(err.to_string().contains("window `error`"));
    }

    #[test]
    fn windows_expose_registered_window_metadata_and_open_state() {
        let services = EditorServices::new();

        let first = services.add_window_fn(
            EditorWindowDescriptor {
                title: "zeta".to_owned(),
                category: "Universe".to_owned(),
                default_open: true,
                show_in_list: true,
            },
            |_ui, _context| Ok(()),
        );
        let second = services.add_window_fn(
            EditorWindowDescriptor {
                title: "alpha".to_owned(),
                category: "Editor".to_owned(),
                default_open: false,
                show_in_list: false,
            },
            |_ui, _context| Ok(()),
        );

        assert_eq!(
            services.windows(),
            vec![
                EditorWindowInfo {
                    id: first,
                    title: "zeta".to_owned(),
                    category: "Universe".to_owned(),
                    open: true,
                    show_in_list: true,
                },
                EditorWindowInfo {
                    id: second,
                    title: "alpha".to_owned(),
                    category: "Editor".to_owned(),
                    open: false,
                    show_in_list: false,
                },
            ]
        );
        assert_eq!(
            services.window(second),
            Some(EditorWindowInfo {
                id: second,
                title: "alpha".to_owned(),
                category: "Editor".to_owned(),
                open: false,
                show_in_list: false,
            })
        );
    }

    #[test]
    fn editor_open_window_command_reopens_closed_windows() {
        let services = EditorServices::new();
        let id = services.add_window_fn(descriptor("window", false), |_ui, _context| Ok(()));

        services.open_window_for_tests(id);

        assert_eq!(services.is_open(id), Some(true));
    }

    #[test]
    fn windows_can_request_other_windows_to_open_during_render() -> anyhow::Result<()> {
        let services = EditorServices::new();
        let target = services.add_window_fn(descriptor("target", false), |_ui, _context| Ok(()));
        services.add_window_fn(descriptor("source", true), move |_ui, context| {
            context.open_window(target);
            Ok(())
        });

        let universe = Universe::builder().build();
        render_services(&services, &universe)?;

        assert_eq!(services.is_open(target), Some(true));
        Ok(())
    }

    #[test]
    fn menu_capabilities_are_registered_in_registration_order() {
        let services = EditorServices::new();

        for label in ["first", "second", "third"] {
            services.add_menu_fn(
                EditorMenuDescriptor {
                    title: label.to_owned(),
                },
                move |_ui, _context, editor| {
                    let _ = editor.windows();
                    Ok(())
                },
            );
        }

        assert_eq!(services.menu_titles(), vec!["first", "second", "third"]);
    }
}
