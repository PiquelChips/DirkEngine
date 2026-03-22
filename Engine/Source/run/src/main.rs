mod test;

fn run() -> anyhow::Result<()> {
    let logger = logging::Logger::new(true, true, true);
    logging::init(logger);

    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => panic!("Error: {err:#}"),
    }
}
