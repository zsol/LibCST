use std::{borrow::Cow, iter};

use super::*;

macro_rules! lit {
    ($e:literal) => {
        Token { string: $e, .. }
    };
}

macro_rules! tok {
    ($e:ident) => {
        Token {
            r#type: TokType::$e,
            ..
        }
    };
}

#[derive(Debug)]
pub struct TokVec<'a>(Vec<Token<'a>>);

impl<'a> Into<TokVec<'a>> for Vec<Token<'a>> {
    fn into(self) -> TokVec<'a> {
        TokVec(self)
    }
}

impl<'a> Parse for TokVec<'a> {
    type PositionRepr = usize;

    fn start(&self) -> usize {
        0
    }

    fn is_eof(&self, pos: usize) -> bool {
        pos >= self.0.len()
    }

    fn position_repr(&self, pos: usize) -> Self::PositionRepr {
        pos
    }
}

impl<'a> ParseElem for TokVec<'a> {
    type Element = Token<'a>;

    fn parse_elem(&self, pos: usize) -> RuleResult<Self::Element> {
        match self.0.get(pos) {
            Some(tok) => RuleResult::Matched(pos + 1, tok.clone()),
            None => RuleResult::Failed,
        }
    }
}

impl<'a> ParseLiteral for TokVec<'a> {
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()> {
        match self.parse_elem(pos) {
            RuleResult::Matched(p, Token { string: lit, .. }) if lit == literal => {
                RuleResult::Matched(p, ())
            }
            _ => RuleResult::Failed,
        }
    }
}

peg::parser! {
    pub grammar python<'a>(config: &Config<'a>) for TokVec<'a> {
        pub rule file() -> Module<'a>
            = s:statements() [tok!(EndMarker)] { Module { body: s } }
        pub rule statements() -> Vec<Statement<'a>>
            = s:statement()+ {s.into_iter().flatten().collect()}
        pub rule statement() -> Vec<Statement<'a>>
            = c:compound_stmt() {vec![c]}
            / simple_stmt() { todo!() }

        rule simple_stmt() -> SimpleStatementSuite<'a>
            // Can't use ** here because need to capture the whitespace for
            // each separator
            = init:(
                s:small_stmt() [semi@lit!(";")] {?
                    add_semi_to_small_stmt(config, s, Some(semi)).map_err(|e| "add_semi_to_small_stmt")
                })*
                // ugly semi syntax :(
                last:(s:small_stmt() semi:([semi@Token{string: ";", ..}]{semi})? {?
                    add_semi_to_small_stmt(config, s, semi).map_err(|e| "add_semi_to_small_stmt")
                }) {? make_simple_stmt_suite(config, init, last).map_err(|e| "expected simple stmt suite") }

        rule small_stmt() -> SmallStatement<'a>
            = "pass" { SmallStatement::Pass() }
            / e:star_expressions() { SmallStatement::Expr(Expr { value: e })}

        rule star_expressions() -> Expression<'a>
            = "..." { Expression::Ellipsis(Default::default()) }

        rule compound_stmt() -> Statement<'a>
            = &("def" / "@" / [Async]) f:function_def() { Statement::FunctionDef(f) }

        rule function_def() -> FunctionDef<'a>
            = d:decorators() f:function_def_raw() {f.with_decorators(d)}
            / function_def_raw()

        rule decorators() -> Vec<Decorator<'a>>
            = ([at@lit!("@")] [name@tok!(Name)] [tok!(Newline)] {? make_decorator(&config, at, name).map_err(|e| "expected decorator")} )+

        rule function_def_raw() -> FunctionDef<'a>
            = [def@lit!("def")] [n@tok!(Name)]
                [op@lit!("(")] params:params()? [cp@lit!(")")]
                [c@lit!(":")]
                b:block()
                {?
                    make_function_def(&config, def, n, op, params, cp, c, b).map_err(|e| "ohno" )
                }

        rule params() -> Parameters<'a>
            = parameters()

        rule parameters() -> Parameters<'a>
            = a:slash_no_default() b:param_no_default()* // c:param_with_default()* d:star_etc()? {}
            { Parameters { params: {a.into_iter().chain(b.into_iter()).collect()}} }
            / a:param_no_default()+ { Parameters {params: a}}

        rule slash_no_default() -> Vec<Param<'a>>
            = a:param_no_default()+ "/" "," { a }
            / a:param_no_default()+ "/" &")" { a }

        rule param_no_default() -> Param<'a>
            = a:param() "," {a}
            / a:param() &")" {a}

        rule param() -> Param<'a>
            = [n@tok!(Name)] { Param {name: Name {value: n.string}, whitespace_after_param: SimpleWhitespace(""), whitespace_after_star: SimpleWhitespace("")}}

        rule block() -> Suite<'a>
            = [nl@tok!(Newline)] [i@tok!(Indent)] s:statements() [d@tok!(Dedent)] {? make_indented_block(config, nl, i, s, d).map_err(|e| "expected indented block") }
            / s:simple_stmt() { Suite::SimpleStatementSuite(s) }
    }
}

fn make_function_def<'a>(
    config: &Config<'a>,
    mut def: Token<'a>,
    mut name: Token<'a>,
    open_paren: Token<'a>,
    params: Option<Parameters<'a>>,
    close_paren: Token<'a>,
    mut colon: Token<'a>,
    block: Suite<'a>,
) -> Result<'a, FunctionDef<'a>> {
    Ok(FunctionDef {
        name: Name { value: name.string },
        params: params.unwrap_or_default(),
        body: block,
        decorators: Default::default(),
        whitespace_after_def: parse_simple_whitespace(config, &mut def.whitespace_after)?,
        whitespace_after_name: parse_simple_whitespace(config, &mut name.whitespace_after)?,
        whitespace_before_colon: parse_simple_whitespace(config, &mut colon.whitespace_before)?,
    })
}

fn make_decorator<'a>(
    config: &Config<'a>,
    mut at: Token<'a>,
    mut name: Token<'a>,
    // mut newline: Token<'a>,
) -> Result<'a, Decorator<'a>> {
    Ok(Decorator {
        decorator: Name { value: name.string },
        leading_lines: parse_empty_lines(config, &mut at.whitespace_before, None)?,
        whitespace_after_at: parse_simple_whitespace(config, &mut at.whitespace_after)?,
        trailing_whitespace: Default::default(), //parse_trailing_whitespace(config, &mut newline.whitespace_before)?,
    })
}

fn make_indented_block<'a>(
    config: &Config<'a>,
    mut nl: Token<'a>,
    mut indent: Token<'a>,
    s: Vec<Statement<'a>>,
    mut dedent: Token<'a>,
) -> Result<'a, Suite<'a>> {
    // We want to be able to only keep comments in the footer that are actually for
    // this IndentedBlock. We do so by assuming that lines which are indented to the
    // same level as the block itself are comments that go at the footer of the
    // block. Comments that are indented to less than this indent are assumed to
    // belong to the next line of code. We override the indent here because the
    // dedent node's absolute indent is the resulting indentation after the dedent
    // is performed. Its this way because the whitespace state for both the dedent's
    // whitespace_after and the next BaseCompoundStatement's whitespace_before is
    // shared. This allows us to partially parse here and parse the rest of the
    // whitespace and comments on the next line, effectively making sure that
    // comments are attached to the correct node.
    // TODO: override indent
    let footer = parse_empty_lines(config, &mut dedent.whitespace_after, None)?;
    Ok(Suite::IndentedBlock(IndentedBlock {
        body: s,
        header: parse_trailing_whitespace(config, &mut nl.whitespace_after)?,
        indent: indent.relative_indent,
        footer,
    }))
}

fn make_simple_stmt_suite<'a>(
    config: &Config<'a>,
    init: Vec<SmallStatement<'a>>,
    last: SmallStatement<'a>,
) -> Result<'a, SimpleStatementSuite<'a>> {
    Ok(SimpleStatementSuite {
        body: init.into_iter().chain(iter::once(last)).collect(),
        ..Default::default()
    })
}

fn add_semi_to_small_stmt<'a>(
    config: &Config<'a>,
    stmt: SmallStatement<'a>,
    semi: Option<Token<'a>>,
) -> Result<'a, SmallStatement<'a>> {
    Ok(stmt) // TODO
}
