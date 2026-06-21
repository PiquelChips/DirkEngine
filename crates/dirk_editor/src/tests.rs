use std::sync::{Arc, OnceLock};

use dirk_engine::editor::EditorSubsystem as _;

fn test_handle() -> dirk_engine::EngineHandle {
    static HANDLE: OnceLock<dirk_engine::EngineHandle> = OnceLock::new();

    HANDLE
        .get_or_init(|| {
            struct HandleCapture;

            impl dirk_engine::Subsystem for HandleCapture {
                fn name(&self) -> &'static str {
                    "handle-capture"
                }
            }

            let captured = Arc::new(parking_lot::Mutex::new(None));
            let observed = Arc::clone(&captured);
            let mut builder = dirk_engine::Engine::builder();
            builder.add_subsystem(move |ctx| {
                *observed.lock() = Some(ctx.handle().clone());
                Ok(HandleCapture)
            });
            let _engine = builder.build().expect("engine should build");
            captured
                .lock()
                .clone()
                .expect("handle should be captured while building")
        })
        .clone()
}

#[test]
fn builtin_editor_subsystem_registers_expected_default_capabilities() -> anyhow::Result<()> {
    let services = crate::EditorServices::new();
    let handle = test_handle();
    let universe = dirk_universe::Universe::builder().build();
    let mut subsystem = crate::BuiltinEditorSubsystem;
    let mut context = dirk_engine::editor::EditorStartContext {
        engine: &handle,
        universe: &universe,
        editor: &services,
    };

    subsystem.start(&mut context)?;

    assert_eq!(services.menu_titles(), vec!["Main", "Settings", "Windows"]);
    assert_eq!(
        services.window_titles(),
        vec!["Settings", "Engine", "Worlds", "Entities", "Entity Details",]
    );

    Ok(())
}

#[test]
fn window_list_menu_groups_categories_and_windows_alphabetically() {
    let services = crate::EditorServices::new();

    services.add_window_fn(
        crate::EditorWindowDescriptor {
            title: "zeta".to_owned(),
            category: "Universe".to_owned(),
            default_open: true,
            show_in_list: true,
        },
        |_ui, _context| Ok(()),
    );
    services.add_window_fn(
        crate::EditorWindowDescriptor {
            title: "alpha".to_owned(),
            category: "Editor".to_owned(),
            default_open: true,
            show_in_list: true,
        },
        |_ui, _context| Ok(()),
    );
    services.add_window_fn(
        crate::EditorWindowDescriptor {
            title: "beta".to_owned(),
            category: "Editor".to_owned(),
            default_open: true,
            show_in_list: true,
        },
        |_ui, _context| Ok(()),
    );
    services.add_window_fn(
        crate::EditorWindowDescriptor {
            title: "alpha".to_owned(),
            category: "Universe".to_owned(),
            default_open: true,
            show_in_list: true,
        },
        |_ui, _context| Ok(()),
    );
    services.add_window_fn(
        crate::EditorWindowDescriptor {
            title: "hidden".to_owned(),
            category: "Settings".to_owned(),
            default_open: false,
            show_in_list: false,
        },
        |_ui, _context| Ok(()),
    );

    let grouped_titles = crate::grouped_window_menu_entries(&services.windows())
        .into_iter()
        .map(|(category, windows)| {
            (
                category,
                windows
                    .into_iter()
                    .map(|window| window.title)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        grouped_titles,
        vec![
            (
                "Editor".to_owned(),
                vec!["alpha".to_owned(), "beta".to_owned()],
            ),
            (
                "Universe".to_owned(),
                vec!["alpha".to_owned(), "zeta".to_owned()],
            ),
        ]
    );
}

#[test]
fn editor_plugin_registers_builtin_editor_subsystem() -> anyhow::Result<()> {
    struct Observer;

    impl dirk_engine::editor::EditorSubsystem for Observer {
        fn name(&self) -> &'static str {
            "observer"
        }
    }

    let observed = Arc::new(parking_lot::Mutex::new(None));
    let observed_services = Arc::clone(&observed);
    let mut builder = dirk_engine::Engine::builder();

    builder.with_plugin(crate::EditorPlugin)?;
    builder.add_editor_subsystem(move |ctx| {
        *observed_services.lock() = Some(ctx.engine.resource::<crate::EditorServices>()?);
        Ok(Observer)
    });

    let mut engine = builder.build()?;
    engine.start()?;

    let services = observed
        .lock()
        .clone()
        .expect("observer should capture editor services");

    assert!(services.window_titles().contains(&"Engine".to_owned()));
    Ok(())
}
