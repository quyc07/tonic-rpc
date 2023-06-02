fn main() {
    tonic_build::configure()
        .out_dir("src/route_guide")// 自定义生成代码输出目录，方便IDE识别
        .compile(&["proto/route_guide.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    tonic_build::configure()
        .out_dir("src/hello_world")
        .compile(&["proto/helloworld.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
