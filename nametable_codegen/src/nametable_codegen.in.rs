
use syntax::codemap::{Span, respan, dummy_spanned, DUMMY_SP};

use syntax::ptr::P;

use syntax::ast::{TokenTree, Name, Delimited, SyntaxContext, Expr, Ident, StrStyle, LitKind, Item, Visibility,
                  ItemKind, Generics, EnumDef, VariantData, Variant_,
                  Path, PathSegment, PathParameters, DUMMY_NODE_ID};

use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntax::ext::build::AstBuilder;

use syntax::parse::parser::{Parser, PathParsingMode};
use syntax::parse::token::{Token, DelimToken, Lit,
                           intern, intern_and_get_ident};
use syntax::parse::token::keywords::Keyword;

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
            open_span: DUMMY_SP,
            tts: r_inner,
            close_span: DUMMY_SP,
        })));
        r
    }
}

impl ToTokens for MyLiteralString {
    fn to_tokens(&self, _cx: &ExtCtxt) -> Vec<TokenTree> {
		let lit = LitKind::Str(
			intern_and_get_ident(&self.0), StrStyle::Cooked);
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

fn add_suffix_to_path(prefix: &Path, suffix: Ident) -> Path {
    let mut result = prefix.clone();
    result.segments.push(
        PathSegment {
            identifier: suffix,
            parameters: PathParameters::none(),
        }
    );
    result
}

fn add_prefix_to_path(prefix: Ident, suffix: &Path) -> Path {
    let mut result = suffix.clone();
    result.segments.insert(0,
        PathSegment {
            identifier: prefix,
            parameters: PathParameters::none(),
        }
    );
    result
}

fn generate_nametable_item<'cx>(
    cx: &'cx mut ExtCtxt,
    sp: Span,
    sc: SyntaxContext,
    artifact_name: Name,
    base_artifact_path: Option<Path>,
    artifact_items: Vec<(Name, Name)>) -> P<Item> {

    let mod_attributes = Vec::new();
    let mut mod_items = Vec::new();

    let base_artifact_path = base_artifact_path.map(
        |path: Path| if path.global {
            path
        } else {
            add_prefix_to_path(Ident::new(intern("super"), sc), &path)
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
//            mod_items.push(quote_item!(cx, use $path; ).unwrap());
            let base_artifact_initial = add_suffix_to_path(path, Ident::new(intern("INITIAL"), sc));
            let base_artifact_count = add_suffix_to_path(path, Ident::new(intern("COUNT"), sc));
            //            mod_items.push(quote_item!(cx, const INITIAL : usize = $path::INITIAL + $path::COUNT; ).unwrap());
            mod_items.push(quote_item!(cx, pub const INITIAL : usize = $base_artifact_initial + $base_artifact_count; ).unwrap());
        } else {
            mod_items.push(quote_item!(cx, pub const INITIAL : usize = 0usize; ).unwrap());
    }

        let count = artifact_items.len();
        mod_items.push(quote_item!(cx, pub const COUNT : usize = $count; ).unwrap());
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
                             vec!(cx.meta_word(sp, intern_and_get_ident("Copy")),
                                  cx.meta_word(sp, intern_and_get_ident("Clone")))));
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
        let base_artifact_new = base_artifact_path.map(|path| {
            add_suffix_to_path(&path, Ident::new(intern("new"), sc))
        });
        //functions
        mod_items.push(match base_artifact_new {
            Some(ref path) => {
                quote_item!(
            cx,
            pub fn new<'x>() -> StaticHashedNameTable<'x> {
                        StaticHashedNameTable::new_upon(NAME_DATA, INDEX_DATA, HASH_DATA, $path())
                    }
                )
            },
            None => {
                quote_item!(
                    cx,
                    pub fn new<'x>() -> StaticHashedNameTable<'x> {
                StaticHashedNameTable::new(NAME_DATA, INDEX_DATA, HASH_DATA)
            }
                )
            }
        }.unwrap());

        mod_items.push(quote_item!(
            cx,
            pub fn new_dynamic<'x>() -> DynamicNameTable<'x> {
                DynamicNameTable::new_upon(
                    StaticHashedNameTable::new(NAME_DATA, INDEX_DATA, HASH_DATA))
            }
        ).unwrap());

        mod_items.push(quote_item!(
            cx,
            pub fn new_plain_<'x>() -> StaticNameTable<'x> {
                StaticNameTable::new(NAME_DATA, INDEX_DATA)
            }
        ).unwrap());

        mod_items.push(quote_item!(
            cx,
            pub fn new_dynamic_plain<'x>() -> DynamicNameTable<'x> {
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

        let base_artifact_path = if parser.eat(&Token::Colon) {
            let result = parser.parse_path(PathParsingMode::NoTypesAllowed);
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

