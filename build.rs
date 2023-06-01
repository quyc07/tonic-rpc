fn main() {
    tonic_build::compile_protos("proto/helloworld.proto")
    .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
tonic_build::compile_protos("proto/route_guide.proto")
    .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}