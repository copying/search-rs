fn main() {
    tonic_build::compile_protos("proto/searchindex.proto").unwrap();
}
