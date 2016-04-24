extern crate syntex;
extern crate nametable_codegen;

use std::env;
use std::path::Path;

fn main() {
    let mut registry = syntex::Registry::new();
    nametable_codegen::register(&mut registry);

    let src = Path::new("tests/tables.in.rs");
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("tables.rs");

    registry.expand("nametable_codegen_test", &src, &dst).unwrap();
}
