// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::cmp::{max, min};

mod tokenizer;

use tokenizer::{whitespace_parser, TokConfig, Token, TokenIterator};

mod nodes;
pub use nodes::*;

mod parser;
use parser::{ParserError, Result};

pub mod py;

pub fn tokenize(text: &str) -> Result<Vec<Token>> {
    let iter = TokenIterator::new(
        text,
        &TokConfig {
            async_hacks: false,
            split_fstring: true,
        },
    );

    iter.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(ParserError::TokenizerError)
}

pub fn parse_tokens_without_whitespace<'a>(
    tokens: Vec<Token<'a>>,
    module_text: &'a str,
) -> Result<'a, Module<'a>> {
    parser::python::file(&tokens.into(), module_text).map_err(ParserError::ParserError)
}

pub fn parse_whitespace<'a, T: Inflate<'a>>(m: T, module_text: &'a str) -> Result<'a, T> {
    let conf = whitespace_parser::Config::new(module_text);
    Ok(m.inflate(&conf)?)
}

pub fn parse_module(mut module_text: &str) -> Result<Module> {
    // Strip UTF-8 BOM
    if let Some(stripped) = module_text.strip_prefix('\u{feff}') {
        module_text = stripped;
    }
    // eprintln!("{:#?}", result);
    let tokens = tokenize(module_text)?;
    let m = parse_tokens_without_whitespace(tokens, module_text)?;
    parse_whitespace(m, module_text)
}

pub fn parse_statement(text: &str) -> Result<Statement> {
    let mut tokens = tokenize(text)?;
    // HACK: we don't need an EOF token for this term
    tokens.pop();
    let stm = parser::python::statement(&tokens.into(), text).map_err(ParserError::ParserError)?;
    parse_whitespace(stm, text)
}

pub fn parse_expression(text: &str) -> Result<Expression> {
    let mut tokens = tokenize(text)?;
    // HACK: we don't need an EOF and EOL token for this term
    tokens.pop(); // EOF
    tokens.pop(); // EOL
    let expr =
        parser::python::expression(&tokens.into(), text).map_err(ParserError::ParserError)?;
    parse_whitespace(expr, text)
}

// n starts from 1
fn bol_offset(source: &str, n: i32) -> usize {
    if n <= 1 {
        return 0;
    }
    source
        .match_indices('\n')
        .nth((n - 2) as usize)
        .map(|(index, _)| index + 1)
        .unwrap_or_else(|| source.len())
}

pub fn prettify_error<'a>(
    module_text: &'a str,
    err: ParserError<'a>,
    label: &str,
) -> std::string::String {
    match err {
        ParserError::ParserError(e) => {
            let loc = e.location;
            let context = 1;
            let start_offset = bol_offset(module_text, loc.start_pos.line as i32 - context);
            let end_offset = bol_offset(module_text, loc.end_pos.line as i32 + context + 1);
            let source = &module_text[start_offset..end_offset];
            let start = loc.start_pos.offset - start_offset;
            let end = loc.end_pos.offset - start_offset;
            chic::Error::new(label)
                .error(
                    max(
                        1,
                        loc.start_pos
                            .line
                            .checked_sub(context as usize)
                            .unwrap_or(1),
                    ),
                    start,
                    if start == end {
                        min(end + 1, end_offset - start_offset + 1)
                    } else {
                        end
                    },
                    source,
                    format!(
                        "expected {} {} -> {}",
                        e.expected, loc.start_pos, loc.end_pos
                    ),
                )
                .to_string()
        }
        e => format!("Parse error for {}: {}", label, e),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokenizer::TokError;

    #[test]
    fn test_simple() {
        let n = parse_module("1_");
        assert_eq!(
            n.err().unwrap(),
            ParserError::TokenizerError(TokError::BadDecimal)
        );
    }

    #[test]
    fn test_bare_minimum_funcdef() {
        parse_module("def f(): ...").expect("parse error");
    }

    #[test]
    fn test_funcdef_params() {
        parse_module("def g(a, b): ...").expect("parse error");
    }

    #[test]
    fn bol_offset_first_line() {
        assert_eq!(0, bol_offset("hello", 1));
        assert_eq!(0, bol_offset("hello", 0));
        assert_eq!(0, bol_offset("hello\nhello", 1));
        assert_eq!(0, bol_offset("hello\nhello", 0));
    }

    #[test]
    fn bol_offset_second_line() {
        assert_eq!(5, bol_offset("hello", 2));
        assert_eq!(6, bol_offset("hello\nhello", 2));
        assert_eq!(6, bol_offset("hello\nhello\nhello", 2));
    }

    #[test]
    fn bol_offset_last_line() {
        assert_eq!(5, bol_offset("hello", 3));
        assert_eq!(11, bol_offset("hello\nhello", 3));
        assert_eq!(12, bol_offset("hello\nhello\nhello", 3));
    }
}
