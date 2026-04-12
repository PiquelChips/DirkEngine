fn main() {
    build::configure_editor();

    let log_path = format!("{}/logs", build::get_run_dir());
    println!("cargo:rustc-env=LOG_PATH={log_path}");
}
