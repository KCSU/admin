fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../proto");

    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    config.compile_protos(
        &[
            "../../proto/kcsu/lookup/v1/group_snapshot.proto",
            "../../proto/kcsu/alerts/v1/alert.proto",
        ],
        &["../../proto"],
    )?;
    Ok(())
}
