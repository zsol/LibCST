use super::{Codegen, CodegenState, ParenthesizableWhitespace, SimpleWhitespace};

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Parameters<'a> {
    pub params: Vec<Param<'a>>,
}
#[derive(Debug, Eq, PartialEq, Default)]
pub struct Name<'a> {
    pub value: &'a str,
}

impl<'a> Codegen for Name<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        // TODO: parentheses
        state.add_token(self.value.to_string());
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Param<'a> {
    pub name: Name<'a>,

    pub whitespace_after_star: SimpleWhitespace<'a>,
    pub whitespace_after_param: SimpleWhitespace<'a>,
}

/// Used by various nodes to denote a parenthesized section. This doesn't own
/// the whitespace to the right of it since this is owned by the parent node.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct LeftParen<'a> {
    whitespace_after: ParenthesizableWhitespace<'a>,
}

/// Used by various nodes to denote a parenthesized section. This doesn't own
/// the whitespace to the left of it since this is owned by the parent node.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct RightParen<'a> {
    whitespace_before: ParenthesizableWhitespace<'a>,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Parentheses<'a>(LeftParen<'a>, RightParen<'a>);

#[derive(Debug, PartialEq, Eq)]
pub enum Expression<'a> {
    Ellipsis(Parentheses<'a>),
}
