fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=../../proto");

    prost_build::compile_protos(
        &[
            "../../proto/kcsu/lookup/v1/group_snapshot.proto",
            "../../proto/kcsu/alerts/v1/alert.proto",
        ],
        &["../../proto"],
    )
}
