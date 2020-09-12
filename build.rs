fn main() {
    tonic_build::compile_protos("proto/oxydoro/oxydoro.proto").unwrap();
}
