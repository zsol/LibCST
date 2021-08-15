// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{Codegen, CodegenState};

#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct SimpleWhitespace<'a>(pub &'a str);

impl<'a> Codegen<'a> for SimpleWhitespace<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        state.add_token(self.0);
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Comment<'a>(pub &'a str);

impl<'a> Default for Comment<'a> {
    fn default() -> Self {
        Self("#")
    }
}

impl<'a> Codegen<'a> for Comment<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        state.add_token(self.0);
    }
}

#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Newline<'a>(pub Option<&'a str>, pub Fakeness);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Fakeness {
    Fake,
    Real,
}

impl Default for Fakeness {
    fn default() -> Self {
        Self::Real
    }
}

impl<'a> Codegen<'a> for Newline<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        if let Fakeness::Fake = self.1 {
            return;
        }
        if let Some(value) = self.0 {
            state.add_token(value);
        } else {
            state.add_token(state.default_newline);
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct TrailingWhitespace<'a> {
    pub whitespace: SimpleWhitespace<'a>,
    pub comment: Option<Comment<'a>>,
    pub newline: Newline<'a>,
}

impl<'a> Codegen<'a> for TrailingWhitespace<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        self.whitespace.codegen(state);
        if let Some(comment) = &self.comment {
            comment.codegen(state);
        }
        self.newline.codegen(state);
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EmptyLine<'a> {
    pub indent: bool,
    pub whitespace: SimpleWhitespace<'a>,
    pub comment: Option<Comment<'a>>,
    pub newline: Newline<'a>,
    pub indentation: Option<&'a str>,
}

impl<'a> Codegen<'a> for EmptyLine<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        if self.indent {
            state.add_indent()
        }
        self.whitespace.codegen(state);
        if let Some(comment) = &self.comment {
            comment.codegen(state);
        }
        self.newline.codegen(state);
    }
}

impl<'a> Default for EmptyLine<'a> {
    fn default() -> Self {
        Self {
            indent: true,
            whitespace: Default::default(),
            comment: Default::default(),
            newline: Default::default(),
            indentation: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct ParenthesizedWhitespace<'a> {
    pub first_line: TrailingWhitespace<'a>,
    pub empty_lines: Vec<EmptyLine<'a>>,
    pub indent: bool,
    pub last_line: SimpleWhitespace<'a>,
}

impl<'a> Codegen<'a> for ParenthesizedWhitespace<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        self.first_line.codegen(state);
        for line in &self.empty_lines {
            line.codegen(state);
        }
        if self.indent {
            state.add_indent()
        }
        self.last_line.codegen(state);
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ParenthesizableWhitespace<'a> {
    SimpleWhitespace(SimpleWhitespace<'a>),
    ParenthesizedWhitespace(ParenthesizedWhitespace<'a>),
}

impl<'a> Codegen<'a> for ParenthesizableWhitespace<'a> {
    fn codegen(&'a self, state: &mut CodegenState<'a>) {
        match self {
            Self::SimpleWhitespace(w) => w.codegen(state),
            Self::ParenthesizedWhitespace(w) => w.codegen(state),
        }
    }
}

impl<'a> Default for ParenthesizableWhitespace<'a> {
    fn default() -> Self {
        Self::SimpleWhitespace(SimpleWhitespace(""))
    }
}
