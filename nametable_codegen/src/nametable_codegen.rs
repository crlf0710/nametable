use syntex_syntax::codemap::{Span, respan, DUMMY_SP};

use syntex_syntax::ptr::P;

use syntex_syntax::ast::{TokenTree, Name, Delimited, SyntaxContext, Expr, Ident, Item, Visibility,
                  ItemKind, Generics, EnumDef, VariantData, Variant_, DUMMY_NODE_ID};

use syntex_syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntex_syntax::ext::build::AstBuilder;

use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{Token, DelimToken, Lit,
                           intern, intern_and_get_ident};
use syntex_syntax::parse::token::keywords::Keyword;

use syntex_syntax::util::small_vector::SmallVector;

struct MyLiteralArray<T>(Vec<T>);

use syntax::ext::quote::rt::ToTokens;
use std::rc::Rc;
impl<T: ToTokens> ToTokens for MyLiteralArray<T> {
    fn to_tokens(&self, _cx: &ExtCtxt) -> Vec<TokenTree> {
        let mut r = vec![];
        let mut r_inner = vec![];
        for item in self.0.iter() {
            r_inner.append(&mut item.to_tokens(_cx));
            r_inner.push(TokenTree::Token(DUMMY_SP, Token::Comma));
        }
        r.push(TokenTree::Delimited(DUMMY_SP, Rc::new(Delimited {
            delim: DelimToken::Bracket,
            open_span: DUMMY_SP,
            tts: r_inner,
            close_span: DUMMY_SP,
        })));
        r
    }
}

fn to_pub_item(mut item: Item) -> Item {
    item.vis = Visibility::Public;
    item
}

fn to_pub_item_ptr(item: P<Item>) -> P<Item> {
    item.map(to_pub_item)
}

fn generate_nametable_item<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    sc: SyntaxContext,
    artifact_name: Name,
    base_artifact_name: Option<Name>,
    artifact_items: Vec<(Name, Name)>) -> P<Item> {

    let mod_attributes = Vec::new();
    let mut mod_items = Vec::new();

    {
        // use
        mod_items.push(quote_item!(cx, use ::nametable::nametable::{NameTable, StaticNameTable, DynamicNameTable, NameTableIdx};).unwrap());
    }

    {
        // const INITIAL
        let initial_value = 0usize;
        mod_items.push(quote_item!(cx, const INITIAL : usize = $initial_value; ).unwrap());
    }

    {
        // enum Names
        let ident_names = Ident::new(intern("Names"), sc);

        let mut enumdef = EnumDef { variants: Vec::new() };
        for (idx,&(key, _)) in artifact_items.iter().enumerate() {
            let ident_key = Ident::new(key, sc);
            enumdef.variants.push(respan(sp, Variant_ {
                name: ident_key,
                attrs: Vec::new(),
                data: VariantData::Unit(DUMMY_NODE_ID),
                disr_expr: Some(quote_expr!(cx, INITIAL + $idx)),
            }));
        }

        let repr_attribute = cx.attribute(
            sp, cx.meta_list(sp, intern_and_get_ident("repr"),
                             vec!(cx.meta_word(sp, intern_and_get_ident("usize")))));

        let derive_attribute = cx.attribute(
            sp, cx.meta_list(sp, intern_and_get_ident("derive"),
                             vec!(cx.meta_word(sp, intern_and_get_ident("Copy")), cx.meta_word(sp, intern_and_get_ident("Clone")))));
        mod_items.push(to_pub_item_ptr(cx.item(sp, ident_names, vec!(repr_attribute, derive_attribute), ItemKind::Enum(enumdef, Generics::default()))));
    }

    {
        // impl
        mod_items.push(quote_item!(
            cx,
            impl NameTableIdx for Names {
                fn to_index(&self) -> usize() { *self as usize }
            }
        ).unwrap());
    }

    {
        //data
        let mut name_data : String = String::new();
        let mut index_data : MyLiteralArray<usize> = MyLiteralArray(vec!(0));
        for (_,&(_, value)) in artifact_items.iter().enumerate() {
            name_data.push_str(&*value.as_str());
            index_data.0.push(name_data.len());
        }
        mod_items.push(quote_item!(
            cx,
            const NAME_DATA : &'static str = $name_data;
        ).unwrap());
        mod_items.push(quote_item!(
            cx,
            const INDEX_DATA : &'static [usize] = &$index_data;
        ).unwrap());
    }

    {
        //functions
        mod_items.push(quote_item!(
            cx,
            pub fn new<'x>() -> StaticNameTable<'x> {
                StaticNameTable::new(NAME_DATA, INDEX_DATA)
            }
        ).unwrap());

        mod_items.push(quote_item!(
            cx,
            pub fn new_dynamic<'x>() -> DynamicNameTable<'x> {
                DynamicNameTable::new_upon(
                    StaticNameTable::new(NAME_DATA, INDEX_DATA))
            }
        ).unwrap());
    }

    let result = cx.item_mod(sp, sp, Ident::new(artifact_name, sc), mod_attributes, mod_items);

    return result;
}

fn process_nametables<'cx>(cx: &'cx mut ExtCtxt, sp: Span, mut parser: Parser) -> SmallVector<P<Item>> {
    let mut result : Vec<P<Item>> = Vec::new();

    while &parser.token != &Token::Eof {
        let syntax_ctx = if let Token::Ident(nt_keyword, _) = parser.token {
            if nt_keyword.name.as_str() != "nametable" {
                cx.span_fatal(parser.span, "expected keyword `nametable' here.");
            }
            let _ = parser.bump();
            nt_keyword.ctxt
        }
        else {
            cx.span_fatal(parser.span, "expected keyword `nametable' here.");
        };

        let artifact_name = if let Token::Ident(artifact_name, _) = parser.token {
            let _ = parser.bump();
            artifact_name.name
        }
        else {
            cx.span_fatal(parser.span, "expected nametable name here.");
        };

        let base_artifact_name = if parser.eat(&Token::Colon) {
            Some(if let Token::Ident(base_artifact_name, _) = parser.token {
                let _ = parser.bump();
                base_artifact_name.name
            } else {
                cx.span_fatal(parser.span, "expected base nametable name here.");
            })
        } else {
            None
        };

        if !parser.eat(&Token::OpenDelim(DelimToken::Brace)) {
            cx.span_fatal(parser.span, "expected open brace here.");
        }

        let mut artifact_items : Vec<(Name, Name)> = Vec::new();

        while let Token::Ident(item_name, _) = parser.token {
            let _ = parser.bump();
            let item_string = if parser.token.is_keyword(Keyword::For) {
                let _ = parser.bump();
                if let Token::Literal(Lit::Str_(item_string_name), _) = parser.token {
                    let _ = parser.bump();
                    item_string_name
                } else {
                    cx.span_fatal(parser.span, "expected string literal here.");
                }
            } else {
                item_name.name
            };

            artifact_items.push((item_name.name,item_string));

            if !parser.eat(&Token::Comma) {
                break;
            }
        }

        if !parser.eat(&Token::CloseDelim(DelimToken::Brace)) {
            cx.span_fatal(parser.span, "expected close brace here.");
        }

        result.push(generate_nametable_item(cx, sp, syntax_ctx, artifact_name, base_artifact_name, artifact_items));
    }

    if &parser.token != &Token::Eof {
        cx.span_fatal(parser.span, "expected end of `nametable!` macro invocation");
    }

    match result.len() {
        0 => SmallVector::zero(),
        1 => SmallVector::one(result.pop().unwrap()),
        _ => SmallVector::many(result),
    }
}

pub fn expand<'cx>(cx: &'cx mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> Box<MacResult + 'cx> {
    let parser = cx.new_parser_from_tts(tts);

    MacEager::items(process_nametables(cx, sp, parser))
}

