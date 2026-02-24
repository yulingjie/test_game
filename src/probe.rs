use wasmtime::component::bindgen;
bindgen!({ world: "game-world", path: "wit/game.wit" });
// 故意使用不存在的名称，让编译器报错并提示正确名称
fn _probe() {
    let _: NONEXISTENT_TYPE_TO_PROBE_NAMES = todo!();
}
