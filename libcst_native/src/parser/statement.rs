use super::{
    Codegen, CodegenState, EmptyLine, Expression, Name, Parameters, Semicolon, SimpleWhitespace,
    TrailingWhitespace,
};

#[derive(Debug, Eq, PartialEq)]
pub enum Statement<'a> {
    FunctionDef(FunctionDef<'a>),
    Pass,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Suite<'a> {
    IndentedBlock(IndentedBlock<'a>),
    SimpleStatementSuite(SimpleStatementSuite<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SmallStatement<'a> {
    Expr(Expr<'a>),
    Pass(), // TODO: semicolon
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expr<'a> {
    pub value: Expression<'a>,
    // TODO
    // semicolon: Option<
}

#[derive(Debug, Eq, PartialEq)]
pub struct IndentedBlock<'a> {
    /// Sequence of statements belonging to this indented block.
    pub body: Vec<Statement<'a>>,
    /// Any optional trailing comment and the final ``NEWLINE`` at the end of the line.
    pub header: TrailingWhitespace<'a>,
    /// A string represents a specific indentation. A ``None`` value uses the modules's
    /// default indentation. This is included because indentation is allowed to be
    /// inconsistent across a file, just not ambiguously.
    pub indent: Option<&'a str>,
    /// Any trailing comments or lines after the dedent that are owned by this indented
    /// block. Statements own preceeding and same-line trailing comments, but not
    /// trailing lines, so it falls on :class:`IndentedBlock` to own it. In the case
    /// that a statement follows an :class:`IndentedBlock`, that statement will own the
    /// comments and lines that are at the same indent as the statement, and this
    /// :class:`IndentedBlock` will own the comments and lines that are indented
    /// further.
    pub footer: Vec<EmptyLine<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SimpleStatementSuite<'a> {
    /// Sequence of small statements. All but the last statement are required to have
    /// a semicolon.
    pub body: Vec<SmallStatement<'a>>,

    /// The whitespace between the colon in the parent statement and the body.
    pub leading_whitespace: SimpleWhitespace<'a>,
    /// Any optional trailing comment and the final ``NEWLINE`` at the end of the line.
    pub trailing_whitespace: TrailingWhitespace<'a>,
}

impl<'a> Default for SimpleStatementSuite<'a> {
    fn default() -> Self {
        Self {
            body: Default::default(),
            leading_whitespace: SimpleWhitespace(" "),
            trailing_whitespace: Default::default(),
        }
    }
}

impl<'a> Codegen for Statement<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        match &self {
            &Self::Pass => state.add_token("pass".to_string()),
            &Self::FunctionDef(f) => f.codegen(state),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionDef<'a> {
    /// The function name.
    pub name: Name<'a>,
    /// The function parameters. Present even if there are no params.
    pub params: Parameters<'a>,
    /// The function body.
    pub body: Suite<'a>,
    /// Sequence of decorators applied to this function. Decorators are listed in
    /// order that they appear in source (top to bottom) as apposed to the order
    /// that they are applied to the function at runtime.
    pub decorators: Vec<Decorator<'a>>,

    pub whitespace_after_def: SimpleWhitespace<'a>,
    pub whitespace_after_name: SimpleWhitespace<'a>,
    pub whitespace_before_colon: SimpleWhitespace<'a>,
}

impl<'a> FunctionDef<'a> {
    pub fn with_decorators(self, decs: Vec<Decorator<'a>>) -> Self {
        Self {
            decorators: self
                .decorators
                .into_iter()
                .chain(decs.into_iter())
                .collect(),
            ..self
        }
    }
}

impl<'a> Codegen for FunctionDef<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        // TODO: leading lines
        for dec in self.decorators.iter() {
            dec.codegen(state);
        }
        // TODO: lines_after_decorators
        state.add_indent();

        // TODO: async
        state.add_token("def".to_string());
        self.whitespace_after_def.codegen(state);
        self.name.codegen(state);
        self.whitespace_after_name.codegen(state);
        state.add_token("(".to_string());
        // TODO: params
        state.add_token(")".to_string());
        // TODO: returns
        self.whitespace_before_colon.codegen(state);
        state.add_token(":".to_string());
        // TODO: body
        state.add_token(" ...".to_string());
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Decorator<'a> {
    pub decorator: Name<'a>,
    pub leading_lines: Vec<EmptyLine<'a>>,
    pub whitespace_after_at: SimpleWhitespace<'a>,
    pub trailing_whitespace: TrailingWhitespace<'a>,
}

impl<'a> Codegen for Decorator<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        for ll in self.leading_lines.iter() {
            ll.codegen(state);
        }
        state.add_indent();
        state.add_token("@".to_string());
        self.whitespace_after_at.codegen(state);
        self.decorator.codegen(state);
        self.trailing_whitespace.codegen(state);
    }
}
