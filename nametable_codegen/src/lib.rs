extern crate syntex;
extern crate syntex_syntax;
extern crate nametable;

mod nametable_codegen;

use syntex::Registry;

pub fn register(reg: &mut Registry) {
    reg.add_macro("nametable", nametable_macros::expand);
}
