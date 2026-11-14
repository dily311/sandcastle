//! 프론트 바인딩 생성: `cargo run --bin export-bindings`
fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/bindings.ts");
    sandcastle_lib::export_bindings(&sandcastle_lib::specta_builder(), path).expect("TypeScript 바인딩 생성 실패");
    println!("생성됨: {path}");
}
