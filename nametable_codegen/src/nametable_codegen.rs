use syntex_syntax::ast::TokenTree;
use syntex_syntax::codemap::{Span, DUMMY_SP};

use syntex_syntax::ptr::P;
use syntex_syntax::ast::{Expr,Ident};
use syntex_syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use syntex_syntax::ext::build::AstBuilder;
use syntex_syntax::parse::parser::Parser;

use syntex_syntax::parse::token::{Token, DelimToken, IdentStyle, intern};

use syntex_syntax::util::small_vector::SmallVector;

pub fn expand<'cx>(cx: &'cx mut ExtCtxt, _: Span, tts: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut parser = cx.new_parser_from_tts(tts);
    let syntax_ctx;
    if let Token::Ident(nt_keyword, _) = parser.token {
        if nt_keyword.name.as_str() != "nametable" {
            cx.span_fatal(parser.span, "expected keyword `nametable' here.");
        }
        syntax_ctx = nt_keyword.ctxt;
        let _ = parser.bump();

        if !parser.eat(&Token::OpenDelim(DelimToken::Brace)) {
            cx.span_fatal(parser.span, "expected open brace here.");
        }

        if !parser.eat(&Token::CloseDelim(DelimToken::Brace)) {
            cx.span_fatal(parser.span, "expected close brace here.");
        }
    }
    else {
        cx.span_fatal(parser.span, "expected keyword `nametable' here.");
    }

    let result = cx.item_mod(DUMMY_SP, DUMMY_SP, Ident::new(intern("table"), syntax_ctx), vec!(), vec!()).unwrap();

    if &parser.token != &Token::Eof {
        cx.span_fatal(parser.span, "expected end of `nametable!` macro invocation");
    }
    MacEager::items(SmallVector::one(P(result)))
}


