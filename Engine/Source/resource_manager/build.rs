fn main() {
    let assets_path = "./Engine/Assets";
    println!("cargo:rustc-env=ASSETS_PATH={assets_path}");

    let models_path = format!("{}/models", assets_path);
    println!("cargo:rustc-env=MODELS_PATH={models_path}");
}
