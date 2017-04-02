extern crate syntex;
extern crate quasi_codegen;

use std::env;
use std::path::Path;

fn main() {
    let src = Path::new("src/nametable_codegen.in.rs");
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("nametable_codegen.rs");
    quasi_codegen::expand(src, dst).unwrap();
}
