// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::*;
use peg::str::LineCol;
use TokType::{Async, Dedent, EndMarker, Indent, Name as NameTok, Newline as NL, Number, String};

#[derive(Debug)]
pub struct TokVec<'a>(Vec<Token<'a>>);

impl<'a> Into<TokVec<'a>> for Vec<Token<'a>> {
    fn into(self) -> TokVec<'a> {
        TokVec(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseLoc {
    pub start_pos: LineCol,
    pub end_pos: LineCol,
}

impl std::fmt::Display for ParseLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.start_pos.fmt(f)
    }
}

impl<'a> Parse for TokVec<'a> {
    type PositionRepr = ParseLoc;

    fn start(&self) -> usize {
        0
    }

    fn is_eof(&self, pos: usize) -> bool {
        pos >= self.0.len()
    }

    fn position_repr(&self, pos: usize) -> Self::PositionRepr {
        let tok = &self.0.get(pos).unwrap_or_else(|| self.0.last().unwrap());
        ParseLoc {
            start_pos: LineCol {
                line: tok.start_pos.line_number(),
                column: tok.start_pos.char_column_number(),
                offset: tok.start_pos.byte_idx(),
            },
            end_pos: LineCol {
                line: tok.end_pos.line_number(),
                column: tok.end_pos.char_column_number(),
                offset: tok.end_pos.byte_idx(),
            },
        }
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
            = traced(<_file()>)

        rule _file() -> Module<'a>
            = s:statements() tok(EndMarker, "EOF") { Module { body: s } }

        pub rule statements() -> Vec<Statement<'a>>
            = statement()+

        pub rule statement() -> Statement<'a>
            = c:compound_stmt() { Statement::Compound(c) }
            / s:simple_stmt() {?
                    Ok(Statement::Simple(make_simple_statement_line(&config, s)
                        .map_err(|e| "simple_stmt")?))
            }


        rule simple_stmt() -> SimpleStatementParts<'a>
            = first:&_ statements:(s:small_stmt() semi:lit(";") { (s, semi) })*
            last_statement:small_stmt() last_semi:lit(";")? nl:tok(NL, "NEWLINE") {
                SimpleStatementParts {first, statements, last_statement, last_semi, nl}
            }

        rule compound_stmt() -> CompoundStatement<'a>
            = &("def" / "@" / tok(Async, "ASYNC")) f:function_def() {
                CompoundStatement::FunctionDef(f)
            }
            / &"if" f:if_stmt() { CompoundStatement::If(f) }

        rule small_stmt() -> SmallStatement<'a>
            = e:star_expressions() { SmallStatement::Expr { value: e, semicolon: None } }
            / "pass" { SmallStatement::Pass { semicolon: None } }

        rule block() -> Suite<'a>
            = n:tok(NL, "NEWLINE") ind:tok(Indent, "INDENT") s:statements() ded:tok(Dedent, "DEDENT") {?
                make_indented_block(&config, n, ind, s, ded)
                    .map_err(|e| "indented block")
            }
            / s:simple_stmt() {?
                make_simple_statement_suite(&config, s)
                    .map_err(|e| "simple_stmt suite")
            }

        rule star_expressions() -> Expression<'a>
            = star_expression()

        rule star_expression() -> Expression<'a>
            = // TODO lit!("*") bitwise_or()
            expression()

        rule expression() -> Expression<'a>
            = disjunction()

        rule disjunction() -> Expression<'a>
            = a:conjunction() b:(or:lit("or") inner:conjunction() { (or, inner) })+ {?
                make_binary_op(&config, a, b).map_err(|e| "expected disjunction")
            }
            / conjunction()

        rule conjunction() -> Expression<'a>
            = a:inversion() b:(and:lit("and") inner:inversion() { (and, inner) })+ {?
                make_binary_op(&config, a, b).map_err(|e| "expected conjunction")
            }
            / inversion()

        rule inversion() -> Expression<'a>
            = not:lit("not") a:inversion() {?
                make_unary_op(&config, not, a).map_err(|e| "expected inversion")
            }
            / comparison()

        rule comparison() -> Expression<'a>
            = a:bitwise_or() b:compare_op_bitwise_or_pair()+ {?
                make_comparison(&config, a, b).map_err(|e| "expected comparison")

            }
            / bitwise_or()

        rule compare_op_bitwise_or_pair() -> (Token<'a>, Expression<'a>)
            = eq_bitwise_or()
            / noteq_bitwise_or()

        rule eq_bitwise_or() -> (Token<'a>, Expression<'a>)
            = op:lit("==") e:bitwise_or() { (op, e) }

        rule noteq_bitwise_or() -> (Token<'a>, Expression<'a>)
            = op:lit("!=") e:bitwise_or() { (op, e) } // TODO: support barry_as_flufl

        rule bitwise_or() -> Expression<'a>
            // TODO left-recursive grammar
            = a:bitwise_xor() tail:(op:lit("|") b:bitwise_xor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected bitwise_or")
            }
            / bitwise_xor()

        rule bitwise_xor() -> Expression<'a>
            // TODO left-recursive grammar
            = a:bitwise_and() tail:(op:lit("^") b:bitwise_and() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected bitwise_xor")
            }
            / bitwise_and()

        rule bitwise_and() -> Expression<'a>
            // TODO left-recursive grammar
            = a:shift_expr() tail:(op:lit("&") b:shift_expr() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected bitwise_and")
            }
            / shift_expr()

        rule shift_expr() -> Expression<'a>
            // TODO left-recursive grammar
            = a:sum() tail:(op:lit("<<") b:shift_expr() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected shift_expr")
            }
            / a:sum() tail:(op:lit(">>") b:shift_expr() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected shift_expr")
            }
            / sum()

        rule sum() -> Expression<'a>
            // TODO left-recursive grammar
            = a:term() tail:(op:lit("+") b:term() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected sum")
            }
            / a:term() tail:(op:lit("-") b:term() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected sum")
            }
            / term()

        rule term() -> Expression<'a>
            // TODO left-recursive grammar
            = a:factor() tail:(op:lit("*") b:factor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected term")
            }
            / a:factor() tail:(op:lit("/") b:factor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected term")
            }
            / a:factor() tail:(op:lit("//") b:factor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected term")
            }
            / a:factor() tail:(op:lit("%") b:factor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected term")
            }
            / a:factor() tail:(op:lit("@") b:factor() {(op, b)})+ {?
                make_binary_op(&config, a, tail).map_err(|e| "expected term")
            }
            / factor()

        rule factor() -> Expression<'a>
            = op:lit("+") a:factor() {?
                make_unary_op(&config, op, a).map_err(|e| "expected factor")
            }
            / op:lit("-") a:factor() {?
                make_unary_op(&config, op, a).map_err(|e| "expected factor")
            }
            / op:lit("~") a:factor() {?
                make_unary_op(&config, op, a).map_err(|e| "expected factor")
            }
            / power()

        rule power() -> Expression<'a>
            = a:await_primary() op:lit("**") b:factor() {?
                make_binary_op(&config, a, vec![(op, b)]).map_err(|e| "expected power")
            }
            / await_primary()

        rule await_primary() -> Expression<'a>
            // TODO: await expressions
            = primary()

        rule primary() -> Expression<'a>
            = atom()
            // TODO: missing left-recursive branches here

        rule atom() -> Expression<'a>
            = n:lit("name") { Expression::Name(Name { value: n.string, lpar: vec![], rpar: vec![] }) }
            / &tok(String, "STRING") s:strings() {s}
            / n:tok(Number, "NUMBER") {? make_number(&config, n).map_err(|e| "expected number")}
            / lit("...") { Expression::Ellipsis {lpar: vec![], rpar: vec![]}}

        rule strings() -> Expression<'a>
            = s:tok(String, "STRING") {
                Expression::SimpleString { value: s.string, lpar: vec![], rpar: vec![]}
            }

        rule function_def() -> FunctionDef<'a>
            = d:decorators() f:function_def_raw() {f.with_decorators(d)}
            / function_def_raw()

        rule decorators() -> Vec<Decorator<'a>>
            = (at:lit("@") name:tok(NameTok, "NAME") tok(NL, "NEWLINE") {? make_decorator(&config, at, name).map_err(|e| "expected decorator")} )+

        rule function_def_raw() -> FunctionDef<'a>
            = def:lit("def") n:tok(NameTok, "FUNCTION NAME") op:lit("(") params:params()? cp:lit(")") c:lit(":") b:block() {?
                make_function_def(&config, def, n, op, params, cp, c, b).map_err(|e| "function def" )
            }

        rule params() -> Parameters<'a>
            = parameters()

        rule parameters() -> Parameters<'a>
            = a:slash_no_default() b:param_no_default()* c:param_with_default()*  d:star_etc()?
            {? make_parameters(&config, Some(a), concat(b, c), d).map_err(|e| "parameters") }
            / a:slash_with_default() b:param_with_default()* d:star_etc()? {?
                make_parameters(&config, Some(a), b, d)
                    .map_err(|e| "parameters")
            }
            / a:param_no_default()+ b:param_with_default()* d:star_etc()? {?
                make_parameters(&config, None, concat(a, b), d)
                    .map_err(|e| "parameters")
            }
            / a:param_with_default()+ d:star_etc()? {?
                make_parameters(&config, None, a, d)
                    .map_err(|e| "parameters")
            }
            / d:star_etc() {?
                make_parameters(&config, None, vec![], Some(d))
                    .map_err(|e| "parameters")
            }

        rule slash_no_default() -> (Vec<Param<'a>>, ParamSlash<'a>)
            = a:param_no_default()+ slash:lit("/") com:lit(",") {?
                make_comma(&config, com)
                    .map(|c| (a, ParamSlash { comma: Some(c)}))
                    .map_err(|e| "slash_no_default")
            }
            / a:param_no_default()+ slash:lit("/") &")" {
                (a, ParamSlash { comma: None })
            }

        rule slash_with_default() -> (Vec<Param<'a>>, ParamSlash<'a>)
            = a:param_no_default()* b:param_with_default()+ slash:lit("/") com:lit(",") {?
                make_comma(&config, com)
                    .map(|c| (concat(a, b), ParamSlash { comma: Some(c) }))
                    .map_err(|e| "slash_with_default")
            }
            / a:param_no_default()* b:param_with_default()+ slash:lit("/") &")" {
                (concat(a, b), ParamSlash { comma: None })
            }

        rule star_etc() -> StarEtc<'a>
            = star:lit("*") a:param_no_default() b:param_maybe_default()* kw:kwds()? {?
                add_param_star(&config, a, star)
                    .map(|p| StarEtc(Some(StarArg::Param(p)), b, kw))
                    .map_err(|e| "star_etc")
            }
            / star:lit("*") com:lit(",") b:param_maybe_default()+ kw:kwds()? {?
                make_comma(&config, com)
                    .map(|comma| StarEtc(Some(StarArg::Star(ParamStar {comma})), b, kw))
                    .map_err(|e| "star_etc")
            }
            / kw:kwds() { StarEtc(None, vec![], Some(kw)) }

        rule kwds() -> Param<'a>
            = star:lit("**") a:param_no_default() {?
                add_param_star(&config, a, star)
                    .map_err(|e| "kwds")
            }

        rule param_no_default() -> Param<'a>
            = a:param() c:lit(",") {? add_param_default(&config, a, None, Some(c)).map_err(|e| "param_no_default") }
            / a:param() &")" {a}

        rule param_with_default() -> Param<'a>
            = a:param() def:default() c:lit(",") {?
                add_param_default(&config, a, Some(def), Some(c))
                    .map_err(|e| "param_with_default")
            }
            / a:param() def:default() &")" {?
                add_param_default(&config, a, Some(def), None)
                    .map_err(|e| "param_with_default")
            }

        rule param_maybe_default() -> Param<'a>
            = a:param() def:default()? c:lit(",") {?
                add_param_default(&config, a, def, Some(c))
                    .map_err(|e| "param_maybe_default")
            }
            / a:param() def:default()? &")" {?
                add_param_default(&config, a, def, None)
                .map_err(|e| "param_maybe_default")
            }

        rule param() -> Param<'a>
            = n:tok(NameTok, "PARAMETER NAME") { Param {name: Name {value: n.string, ..Default::default()}, ..Default::default()} }

        rule default() -> (AssignEqual<'a>, Expression<'a>)
            = eq:lit("=") ex:expression() {?
                Ok((make_assign_equal(&config, eq).map_err(|e| "=")?, ex))
            }

        rule if_stmt() -> If<'a>
            = i:lit("if") a:named_expression() col:lit(":") b:block() elif:elif_stmt() {?
                make_if(&config, i, a, col, b, Some(OrElse::Elif(elif)), false)
                    .map_err(|e| "if statement")
            }
            / i:lit("if") a:named_expression() col:lit(":") b:block() el:else_block()? {?
                make_if(&config, i, a, col, b, el.map(OrElse::Else), false)
                    .map_err(|e| "if statement")
            }

        rule elif_stmt() -> If<'a>
            = i:lit("elif") a:named_expression() col:lit(":") b:block() elif:elif_stmt() {?
                make_if(&config, i, a, col, b, Some(OrElse::Elif(elif)), true)
                    .map_err(|e| "elif statement")
            }
            / i:lit("elif") a:named_expression() col:lit(":") b:block() el:else_block()? {?
                make_if(&config, i, a, col, b, el.map(OrElse::Else), true)
                    .map_err(|e| "elif statement")
            }

        rule else_block() -> Else<'a>
            = el:lit("else") col:lit(":") b:block() {?
                make_else(&config, el, col, b)
                    .map_err(|e| "else block")
            }

        rule named_expression() -> Expression<'a>
            = a:tok(NameTok, "NAME") op:lit(":=") b:expression() { todo!() }
            / e:expression() !lit(":=") { e }

        rule mb_lit(lit: &str) -> Option<Token<'a>>
            = ([t@Token {..}] {? if t.string == lit {
                                    Ok(t)
                                } else { Err("optional semicolon") }
                              })?

        /// matches any token, not just whitespace
        rule _() -> Token<'a>
            = [t@_] { t }

        rule lit(lit: &'static str) -> Token<'a>
            = [t@Token {..}] {? if t.string == lit { Ok(t) } else { Err(lit) } }


        rule tok(tok: TokType, err: &'static str) -> Token<'a>
            = [t@Token {..}] {? if t.r#type == tok { Ok(t) } else { Err(err) } }

        rule traced<T>(e: rule<T>) -> T =
            &(input:(_)* {
                #[cfg(feature = "trace")]
                {
                    println!("[PEG_INPUT_START]");
                    for tok in input {
                        print!("{}", tok.string);
                    }
                    println!();
                    println!("[PEG_TRACE_START]");
                }
            })
            e:e()? {?
                #[cfg(feature = "trace")]
                println!("[PEG_TRACE_STOP]");
                e.ok_or("")
            }

    }
}

fn make_function_def<'a>(
    config: &Config<'a>,
    mut def: Token<'a>,
    mut name: Token<'a>,
    mut open_paren: Token<'a>,
    params: Option<Parameters<'a>>,
    _close_paren: Token<'a>,
    mut colon: Token<'a>,
    body: Suite<'a>,
) -> Result<'a, FunctionDef<'a>> {
    Ok(FunctionDef {
        name: Name {
            value: name.string,
            ..Default::default()
        },
        params: params.unwrap_or_default(),
        decorators: Default::default(),
        body,
        leading_lines: parse_empty_lines(config, &mut def.whitespace_before, None)?,
        lines_after_decorators: vec![],
        whitespace_after_def: parse_simple_whitespace(config, &mut def.whitespace_after)?,
        whitespace_after_name: parse_simple_whitespace(config, &mut name.whitespace_after)?,
        whitespace_before_colon: parse_simple_whitespace(config, &mut colon.whitespace_before)?,
        whitespace_before_params: parse_parenthesizable_whitespace(
            config,
            &mut open_paren.whitespace_after,
        )?,
    })
}

fn make_decorator<'a>(
    config: &Config<'a>,
    mut at: Token<'a>,
    mut name: Token<'a>,
    // mut newline: Token<'a>,
) -> Result<'a, Decorator<'a>> {
    Ok(Decorator {
        decorator: Name {
            value: name.string,
            ..Default::default()
        },
        leading_lines: parse_empty_lines(config, &mut at.whitespace_before, None)?,
        whitespace_after_at: parse_simple_whitespace(config, &mut at.whitespace_after)?,
        trailing_whitespace: Default::default(), //parse_trailing_whitespace(config, &mut newline.whitespace_before)?,
    })
}

fn make_comparison<'a>(
    config: &Config<'a>,
    head: Expression<'a>,
    tail: Vec<(Token<'a>, Expression<'a>)>,
) -> Result<'a, Expression<'a>> {
    todo!()
}

fn make_binary_op<'a>(
    _config: &Config<'a>,
    head: Expression<'a>,
    tail: Vec<(Token<'a>, Expression<'a>)>,
) -> Result<'a, Expression<'a>> {
    if tail.is_empty() {
        return Ok(head);
    }

    let mut expr = head;
    for (tok, right) in tail {
        expr = Expression::BinaryOperation {
            left: Box::new(expr),
            operator: tok.string,
            right: Box::new(right),
            lpar: vec![],
            rpar: vec![],
        }
    }
    Ok(expr)
}

fn make_unary_op<'a>(
    _config: &Config<'a>,
    op: Token<'a>,
    tail: Expression<'a>,
) -> Result<'a, Expression<'a>> {
    Ok(Expression::UnaryOperation {
        operator: op.string,
        expression: Box::new(tail),
        lpar: vec![],
        rpar: vec![],
    })
}

fn make_number<'a>(_config: &Config<'a>, num: Token<'a>) -> Result<'a, Expression<'a>> {
    Ok(Expression::Integer {
        value: num.string,
        lpar: vec![],
        rpar: vec![],
    })
}

fn make_indented_block<'a>(
    config: &Config<'a>,
    mut nl: Token<'a>,
    indent: Token<'a>,
    statements: Vec<Statement<'a>>,
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
    let footer = parse_empty_lines(
        config,
        &mut dedent.whitespace_after,
        Some(indent.whitespace_before.absolute_indent),
    )?;
    let header = parse_trailing_whitespace(config, &mut nl.whitespace_before)?;
    Ok(Suite::IndentedBlock(IndentedBlock {
        body: statements,
        header,
        indent: indent.relative_indent,
        footer,
    }))
}

struct SimpleStatementParts<'a> {
    first: Token<'a>, // The first token of the first statement. Used for its whitespace
    statements: Vec<(SmallStatement<'a>, Token<'a>)>, // statement, semicolon pairs
    last_statement: SmallStatement<'a>,
    last_semi: Option<Token<'a>>,
    nl: Token<'a>,
}

fn _make_simple_statement<'a>(
    config: &Config<'a>,
    mut parts: SimpleStatementParts<'a>,
) -> Result<'a, (Token<'a>, Vec<SmallStatement<'a>>, TrailingWhitespace<'a>)> {
    let mut body = vec![];
    for (statement, semi) in parts.statements {
        // TODO: parse whitespace before and after semi and attach it to a semicolon
        // inside statement
        body.push(statement);
    }
    // TODO: parse whitespace before last semi and attach it to a semicolon inside
    // last_statement
    body.push(parts.last_statement);

    // TODO: is this correct?
    let trailing_whitespace = parse_trailing_whitespace(config, &mut parts.nl.whitespace_before)?;

    Ok((parts.first, body, trailing_whitespace))
}

fn make_simple_statement_suite<'a>(
    config: &Config<'a>,
    parts: SimpleStatementParts<'a>,
) -> Result<'a, Suite<'a>> {
    let (mut first, body, trailing_whitespace) = _make_simple_statement(config, parts)?;
    let leading_whitespace = parse_simple_whitespace(config, &mut first.whitespace_before)?;
    Ok(Suite::SimpleStatementSuite(SimpleStatementSuite {
        body,
        leading_whitespace,
        trailing_whitespace,
    }))
}

fn make_simple_statement_line<'a>(
    config: &Config<'a>,
    parts: SimpleStatementParts<'a>,
) -> Result<'a, SimpleStatementLine<'a>> {
    let (mut first, body, trailing_whitespace) = _make_simple_statement(config, parts)?;

    let leading_lines = parse_empty_lines(config, &mut first.whitespace_before, None)?;
    Ok(SimpleStatementLine {
        body,
        leading_lines,
        trailing_whitespace,
    })
}

fn make_if<'a>(
    config: &Config<'a>,
    mut keyword: Token<'a>,
    cond: Expression<'a>,
    mut colon: Token<'a>,
    block: Suite<'a>,
    orelse: Option<OrElse<'a>>,
    is_elif: bool,
) -> Result<'a, If<'a>> {
    let leading_lines = parse_empty_lines(config, &mut keyword.whitespace_before, None)?;
    let whitespace_before_test = parse_simple_whitespace(config, &mut keyword.whitespace_after)?;
    let whitespace_after_test = parse_simple_whitespace(config, &mut colon.whitespace_before)?;
    Ok(If {
        leading_lines,
        whitespace_before_test,
        test: cond,
        whitespace_after_test,
        body: block,
        orelse: orelse.map(Box::new),
        is_elif,
    })
}

fn make_else<'a>(
    config: &Config<'a>,
    mut keyword: Token<'a>,
    mut colon: Token<'a>,
    block: Suite<'a>,
) -> Result<'a, Else<'a>> {
    let leading_lines = parse_empty_lines(config, &mut keyword.whitespace_before, None)?;
    let whitespace_before_colon = parse_simple_whitespace(config, &mut colon.whitespace_before)?;
    Ok(Else {
        leading_lines,
        whitespace_before_colon,
        body: block,
    })
}

struct StarEtc<'a>(Option<StarArg<'a>>, Vec<Param<'a>>, Option<Param<'a>>);

fn make_parameters<'a>(
    _config: &Config<'a>,
    posonly: Option<(Vec<Param<'a>>, ParamSlash<'a>)>,
    params: Vec<Param<'a>>,
    star_etc: Option<StarEtc<'a>>,
) -> Result<'a, Parameters<'a>> {
    let (posonly_params, posonly_ind) = match posonly {
        Some((a, b)) => (a, Some(b)),
        None => (vec![], None),
    };
    let (star_arg, kwonly_params, star_kwarg) = match star_etc {
        None => (None, vec![], None),
        Some(StarEtc(a, b, c)) => (a, b, c),
    };
    Ok(Parameters {
        params,
        kwonly_params,
        posonly_params,
        posonly_ind,
        star_kwarg,
        star_arg,
    })
}

fn add_param_default<'a>(
    config: &Config<'a>,
    param: Param<'a>,
    def: Option<(AssignEqual<'a>, Expression<'a>)>,
    comma_tok: Option<Token<'a>>,
) -> Result<'a, Param<'a>> {
    let comma = match comma_tok {
        None => None,
        Some(c) => Some(make_comma(config, c)?),
    };

    let (equal, default) = match def {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };
    Ok(Param {
        comma,
        equal,
        default,
        ..param
    })
}

fn add_param_star<'a>(
    config: &Config<'a>,
    param: Param<'a>,
    mut star: Token<'a>,
) -> Result<'a, Param<'a>> {
    let whitespace_after_star =
        parse_parenthesizable_whitespace(config, &mut star.whitespace_after)?;
    Ok(Param {
        star: Some(star.string),
        whitespace_after_star,
        ..param
    })
}

fn make_assign_equal<'a>(config: &Config<'a>, mut eq: Token<'a>) -> Result<'a, AssignEqual<'a>> {
    let whitespace_before = parse_parenthesizable_whitespace(config, &mut eq.whitespace_before)?;
    let whitespace_after = parse_parenthesizable_whitespace(config, &mut eq.whitespace_after)?;
    Ok(AssignEqual {
        whitespace_after,
        whitespace_before,
    })
}

fn make_comma<'a>(config: &Config<'a>, mut tok: Token<'a>) -> Result<'a, Comma<'a>> {
    let whitespace_before = parse_parenthesizable_whitespace(config, &mut tok.whitespace_before)?;
    let whitespace_after = parse_parenthesizable_whitespace(config, &mut tok.whitespace_after)?;
    Ok(Comma {
        whitespace_before,
        whitespace_after,
    })
}

fn concat<T>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    a.into_iter().chain(b.into_iter()).collect()
}