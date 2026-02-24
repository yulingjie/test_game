// build.rs
//
// 路径 B 的关键步骤：
// wit-bindgen-build 读取 wit/game.wit，在编译期自动生成：
//   - Rust 侧的强类型结构体（KeyboardInput、UpdateResult 等）
//   - Bevy 需要实现的 trait（GameLogicImports）
//   - wasmtime Component 的调用胶水代码
//
// 这完全替代了原来手写的 build_args / parse_output 桥接代码。

fn main() {
    // 告知 cargo：wit 目录变化时重新运行 build.rs
    println!("cargo:rerun-if-changed=wit/");
    println!("cargo:rerun-if-changed=assets/game_logic.wasm");
}
