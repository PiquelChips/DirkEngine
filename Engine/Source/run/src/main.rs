fn run() -> anyhow::Result<()> {
    let mut engine = engine::Engine::init()?;
    while engine.tick()? {}
    engine.shutdown()?;
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => panic!("Error: {err:#}"),
    }
}
