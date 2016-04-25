extern crate syntex;
extern crate syntex_syntax as syntax;
extern crate nametable;
extern crate quasi;

include!(concat!(env!("OUT_DIR"), "/nametable_codegen.rs"));

use syntex::Registry;

pub fn register(reg: &mut Registry) {
    reg.add_macro("nametable", expand);
}
