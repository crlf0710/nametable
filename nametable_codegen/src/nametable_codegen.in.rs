
use syntax::codemap::{Span, respan, dummy_spanned, DUMMY_SP};

use syntax::ptr::P;

use syntax::tokenstream::{TokenTree, Delimited};

use syntax::ext::hygiene::SyntaxContext;

use syntax::ast::{Name, Ident,
                  StrStyle, LitKind, Item, Visibility, Mod,
                  ItemKind, Generics, EnumDef, VariantData, Variant_,
                  Path, PathSegment, DUMMY_NODE_ID,
                  MetaItem, NestedMetaItem, NestedMetaItemKind};

use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntax::ext::build::AstBuilder;

use syntax::parse::parser::{Parser, PathStyle};
use syntax::parse::token::{Token, DelimToken, Lit};

use syntax::symbol::Symbol;
use syntax::symbol::keywords;

use syntax::util::small_vector::SmallVector;

use nametable::name_hash;

struct MyLiteralArray<T>(Vec<T>);
struct MyLiteralString(String);
struct MyLiteralTuple2<T1,T2>(T1,T2);

use quasi::ToTokens;
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
            tts: r_inner,
        })));
        r
    }
}

impl ToTokens for MyLiteralString {
    fn to_tokens(&self, _cx: &ExtCtxt) -> Vec<TokenTree> {
		let lit = LitKind::Str(
			Symbol::intern(&self.0), StrStyle::Cooked);
        dummy_spanned(lit).to_tokens(_cx)
	}
}

impl<T1: ToTokens, T2: ToTokens> ToTokens for MyLiteralTuple2<T1, T2> {
    fn to_tokens(&self, _cx: &ExtCtxt) -> Vec<TokenTree> {
        let mut r = vec![];
        let mut r_inner = vec![];
        r_inner.append(&mut self.0.to_tokens(_cx));
        r_inner.push(TokenTree::Token(DUMMY_SP, Token::Comma));
        r_inner.append(&mut self.1.to_tokens(_cx));
        r.push(TokenTree::Delimited(DUMMY_SP, Rc::new(Delimited {
            delim: DelimToken::Paren,
            tts: r_inner,
        })));
        r
    }
}

fn new_ident(name: Symbol, ctxt: SyntaxContext) -> Ident {
    Ident { name: name, ctxt: ctxt }
}

fn to_pub_item(mut item: Item) -> Item {
    item.vis = Visibility::Public;
    item
}

fn to_pub_item_ptr(item: P<Item>) -> P<Item> {
    item.map(to_pub_item)
}

fn add_suffix_to_path(prefix: &Path, suffix: Ident) -> Path {
    let mut result = prefix.clone();
    result.segments.push(
        PathSegment {
            identifier: suffix,
            parameters: None,
        }
    );
    result
}

fn add_prefix_to_path(prefix: Ident, suffix: &Path) -> Path {
    let mut result = suffix.clone();
    result.segments.insert(0,
        PathSegment {
            identifier: prefix,
            parameters: None,
        }
    );
    result
}

fn dummy_spanned_nested_metaitem(x: MetaItem) -> NestedMetaItem {
    dummy_spanned(NestedMetaItemKind::MetaItem(x))
}

fn generate_nametable_item<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    sc: SyntaxContext,
    artifact_name: Name,
    base_artifact_path: Option<Path>,
    artifact_items: Vec<(Name, Name)>) -> P<Item> {

    let mut mod_attributes = Vec::new();
    let mut mod_items = Vec::new();

    let base_artifact_path = base_artifact_path.map(
        |path: Path|
        if path.segments.len() > 0 {
            let initial_segment_name =
                path.segments[0].identifier.name.as_str();
            if initial_segment_name == Symbol::intern("self").as_str() ||
                initial_segment_name == Symbol::intern("super").as_str() {
                    add_prefix_to_path(new_ident(Symbol::intern("super"), sc), &path)
                } else {
                    path
                }
        } else {
            path
        }
    );

    {
        // use
        mod_items.push(quote_item!(
            cx,
            use ::nametable::{
                NameTable, StaticNameTable, DynamicNameTable,
                StaticHashedNameTable, NameTableIdx};).unwrap());
    }


    {
        // const INITIAL
        if let Some(ref path) = base_artifact_path {
            mod_items.push(cx.item_use_simple(DUMMY_SP, Visibility::Inherited, path.clone()));
            let base_artifact_initial = add_suffix_to_path(path, new_ident(Symbol::intern("INITIAL"), sc));
            let base_artifact_count = add_suffix_to_path(path, new_ident(Symbol::intern("COUNT"), sc));
            mod_items.push(quote_item!(cx, pub const INITIAL : usize = $base_artifact_initial + $base_artifact_count; ).unwrap());
        } else {
            mod_items.push(quote_item!(cx, pub const INITIAL : usize = 0usize; ).unwrap());
    }

        let count = artifact_items.len();
        mod_items.push(quote_item!(cx, pub const COUNT : usize = $count; ).unwrap());
    }

    {
        // enum Names
        let ident_names = new_ident(Symbol::intern("Names"), sc);

        let mut enumdef = EnumDef { variants: Vec::new() };
        for (idx,&(key, _)) in artifact_items.iter().enumerate() {
            let ident_key = new_ident(key, sc);
            enumdef.variants.push(respan(sp, Variant_ {
                name: ident_key,
                attrs: Vec::new(),
                data: VariantData::Unit(DUMMY_NODE_ID),
                disr_expr: Some(quote_expr!(cx, INITIAL + $idx)),
            }));
        }

        let repr_attribute = cx.attribute(
            sp, cx.meta_list(sp, Symbol::intern("repr"),
                             vec!(dummy_spanned_nested_metaitem(cx.meta_word(sp, Symbol::intern("usize"))))));

        let derive_attribute = cx.attribute(
            sp, cx.meta_list(sp, Symbol::intern("derive"),
                             vec!(dummy_spanned_nested_metaitem(cx.meta_word(sp, Symbol::intern("Copy"))),
                                  dummy_spanned_nested_metaitem(cx.meta_word(sp, Symbol::intern("Clone"))))));
        mod_items.push(to_pub_item_ptr(
            cx.item(sp, ident_names,
                    vec!(repr_attribute, derive_attribute),
                    ItemKind::Enum(enumdef, Generics::default()))));
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
        let mut name_data : MyLiteralString = MyLiteralString(String::new());
        let mut index_data : MyLiteralArray<usize> = MyLiteralArray(vec!(0));
        let mut hash_data : MyLiteralArray<MyLiteralTuple2<u64,usize>> = MyLiteralArray(vec!());
        for (idx,&(_, value)) in artifact_items.iter().enumerate() {
            name_data.0.push_str(&*value.as_str());
            index_data.0.push(name_data.0.len());
            hash_data.0.push(MyLiteralTuple2(name_hash(&*value.as_str()),idx));
        }
        hash_data.0.sort_by(|&MyLiteralTuple2(a, _), &MyLiteralTuple2(b, _)| a.cmp(&b));

        fn detect_collision<T: PartialOrd + Copy>(arr: &[MyLiteralTuple2<T, usize>]) -> Option<(usize, usize)> {
            if arr.len() == 0 {
                return None;
            }
            let mut old_val = arr.get(0).unwrap().0;
            for i in 1..arr.len() {
                if arr.get(i).unwrap().0 == old_val {
                    return Some((arr.get(i-1).unwrap().1, arr.get(i).unwrap().1));
                }
                old_val = arr.get(i).unwrap().0;
            }
            return None
        }

        match detect_collision(&hash_data.0) {
            Some((a, b)) => {
                println!(
                    "nametable_macros: Hash collision happened between item index {:} and {:} for table `{:}'",
                    a, b, &*artifact_name.as_str());
                hash_data.0 = vec!();
            },
            None => ()
        }

        mod_items.push(quote_item!(
            cx,
            const NAME_DATA : &'static str = $name_data;
        ).unwrap());
        mod_items.push(quote_item!(
            cx,
            const INDEX_DATA : &'static [usize] = &$index_data;
        ).unwrap());

        mod_items.push(quote_item!(
            cx,
            const HASH_DATA : &'static [(u64,usize)] = &$hash_data;
        ).unwrap());

    }

    {
        //functions
        match base_artifact_path {
            Some(ref path) => {
                let base_artifact_new = add_suffix_to_path(&path, new_ident(Symbol::intern("new"), sc));
                mod_items.push(
                    quote_item!(
                        cx,
                        pub fn new() -> StaticHashedNameTable {
                            StaticHashedNameTable::new_upon(NAME_DATA, INDEX_DATA, HASH_DATA, $base_artifact_new())
                        }).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_dynamic() -> DynamicNameTable {
                        DynamicNameTable::new_upon(
                            StaticHashedNameTable::new_upon(NAME_DATA, INDEX_DATA, HASH_DATA, $base_artifact_new()))
                    }
                ).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_plain() -> StaticNameTable {
                        StaticNameTable::new_upon(NAME_DATA, INDEX_DATA, $base_artifact_new())
                    }
                ).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_dynamic_plain() -> DynamicNameTable {
                        DynamicNameTable::new_upon(
                            StaticNameTable::new_upon(NAME_DATA, INDEX_DATA, $base_artifact_new()))
                    }
                ).unwrap());

            }
            None => {
                mod_items.push(quote_item!(
                        cx,
                        pub fn new() -> StaticHashedNameTable {
                            StaticHashedNameTable::new(NAME_DATA, INDEX_DATA, HASH_DATA)
                        }
                ).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_dynamic() -> DynamicNameTable {
                        DynamicNameTable::new_upon(
                            StaticHashedNameTable::new(NAME_DATA, INDEX_DATA, HASH_DATA))
                    }
                ).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_plain() -> StaticNameTable {
                        StaticNameTable::new(NAME_DATA, INDEX_DATA)
                    }
                ).unwrap());

                mod_items.push(quote_item!(
                    cx,
                    pub fn new_dynamic_plain() -> DynamicNameTable {
                        DynamicNameTable::new_upon(
                            StaticNameTable::new(NAME_DATA, INDEX_DATA))
                    }
                ).unwrap());
            }
        }
    }

      {
          let allow_unused_attribute = cx.attribute(
            sp, cx.meta_list(sp, Symbol::intern("allow"),
                             vec!(dummy_spanned_nested_metaitem(cx.meta_word(sp, Symbol::intern("dead_code"))))));

        let allow_unused_imports_attribute = cx.attribute(
            sp, cx.meta_list(sp, Symbol::intern("allow"),
                             vec!(dummy_spanned_nested_metaitem(cx.meta_word(sp, Symbol::intern("unused_imports"))))));

            mod_attributes.push(allow_unused_attribute);
            mod_attributes.push(allow_unused_imports_attribute);
      }

    P(Item {
        ident: new_ident(artifact_name, sc),
        attrs: mod_attributes,
        id: DUMMY_NODE_ID,
        node: ItemKind::Mod(Mod {
            inner: sp,
            items: mod_items,
        }),
        vis: Visibility::Public,
        span: sp,
    })
}

fn process_nametables<'cx>(cx: &'cx mut ExtCtxt, sp: Span, mut parser: Parser) -> SmallVector<P<Item>> {
    let mut result : Vec<P<Item>> = Vec::new();

    while &parser.token != &Token::Eof {
        let syntax_ctx = if let Token::Ident(nt_keyword) = parser.token {
            if nt_keyword.name != Symbol::intern("nametable") {
                cx.span_fatal(parser.span, "expected keyword `nametable' here.");
            }
            let _ = parser.bump();
            nt_keyword.ctxt
        }
        else {
            cx.span_fatal(parser.span, "expected keyword `nametable' here.");
        };

        let artifact_name = if let Token::Ident(artifact_name) = parser.token {
            let _ = parser.bump();
            artifact_name.name
        }
        else {
            cx.span_fatal(parser.span, "expected nametable name here.");
        };

        let base_artifact_path = if parser.eat(&Token::Colon) {
            let result = parser.parse_path(PathStyle::Expr);
            match result{
                Ok(path) => Some(path),
                _ => cx.span_fatal(parser.span, "expected base nametable name here."),
            }
            } else {
            None
        };

        if !parser.eat(&Token::OpenDelim(DelimToken::Brace)) {
            cx.span_fatal(parser.span, "expected open brace here.");
        }

        let mut artifact_items : Vec<(Name, Name)> = Vec::new();

        while let Token::Ident(item_name) = parser.token {
            let _ = parser.bump();
            let item_string = if parser.token.is_keyword(keywords::For) {
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

        result.push(generate_nametable_item(cx, sp, syntax_ctx, artifact_name, base_artifact_path, artifact_items));
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

