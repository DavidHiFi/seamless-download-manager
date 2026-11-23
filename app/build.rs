fn main() {
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent-dark".into()),
    )
    .expect("the window layout failed to compile");
    embed_resource::compile("assets/sdm.rc", embed_resource::NONE).manifest_required().expect("icon resource");
}
