// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    whitespace::ParenthesizableWhitespace, AssignEqual, Codegen, CodegenState, Comma,
    SimpleWhitespace,
};

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Parameters<'a> {
    pub params: Vec<Param<'a>>,
    pub star_arg: Option<StarArg<'a>>,
    pub kwonly_params: Vec<Param<'a>>,
    pub star_kwarg: Option<Param<'a>>,
    pub posonly_params: Vec<Param<'a>>,
    pub posonly_ind: Option<ParamSlash<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StarArg<'a> {
    Star(ParamStar<'a>),
    Param(Param<'a>),
}

impl<'a> Codegen for Parameters<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        let params_after_kwonly = self.star_kwarg.is_some();
        let params_after_regular = !self.kwonly_params.is_empty() || params_after_kwonly;
        let params_after_posonly = !self.params.is_empty() || params_after_regular;
        let star_included = self.star_arg.is_some() || !self.kwonly_params.is_empty();

        for p in &self.posonly_params {
            p.codegen(state, None, true);
        }

        match &self.posonly_ind {
            Some(ind) => ind.codegen(state, params_after_posonly),
            _ => {
                if !self.posonly_params.is_empty() {
                    if params_after_posonly {
                        state.add_token("/, ".to_string());
                    } else {
                        state.add_token("/".to_string());
                    }
                }
            }
        }

        let param_size = self.params.len();
        for (i, p) in self.params.iter().enumerate() {
            p.codegen(state, None, params_after_regular || i < param_size - 1);
        }

        let kwonly_size = self.kwonly_params.len();
        match &self.star_arg {
            None => {
                if star_included {
                    state.add_token("*, ".to_string())
                }
            }
            Some(StarArg::Param(p)) => p.codegen(
                state,
                Some("*"),
                kwonly_size > 0 || self.star_kwarg.is_some(),
            ),
            Some(StarArg::Star(s)) => s.codegen(state),
        }

        for (i, p) in self.kwonly_params.iter().enumerate() {
            p.codegen(state, None, params_after_kwonly || i < kwonly_size - 1);
        }

        match &self.star_kwarg {
            Some(star) => star.codegen(state, Some("**"), false),
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParamSlash<'a> {
    pub comma: Option<Comma<'a>>,
}

impl<'a> ParamSlash<'a> {
    fn codegen(&self, state: &mut CodegenState, default_comma: bool) -> () {
        state.add_token("/".to_string());
        match (&self.comma, default_comma) {
            (Some(comma), _) => comma.codegen(state),
            (None, true) => state.add_token(", ".to_string()),
            _ => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParamStar<'a> {
    pub comma: Comma<'a>,
}

impl<'a> Codegen for ParamStar<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        state.add_token("*".to_string());
        self.comma.codegen(state);
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Name<'a> {
    pub value: &'a str,
    pub lpar: Vec<LeftParen<'a>>,
    pub rpar: Vec<RightParen<'a>>,
}

impl<'a> Codegen for Name<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        self.parenthesize(state, |state| {
            state.add_token(self.value.to_string());
        });
    }
}

impl<'a> ParenthesizedNode<'a> for Name<'a> {
    fn lpar(&self) -> &Vec<LeftParen<'a>> {
        &self.lpar
    }

    fn rpar(&self) -> &Vec<RightParen<'a>> {
        &self.rpar
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Param<'a> {
    pub name: Name<'a>,
    // TODO: annotation
    pub equal: Option<AssignEqual<'a>>,
    pub default: Option<Expression<'a>>,

    pub comma: Option<Comma<'a>>,

    pub star: Option<&'a str>,

    pub whitespace_after_star: ParenthesizableWhitespace<'a>,
    pub whitespace_after_param: ParenthesizableWhitespace<'a>,
}

impl<'a> Default for Param<'a> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            equal: None,
            default: None,
            comma: None,
            star: None,
            whitespace_after_param: ParenthesizableWhitespace::SimpleWhitespace(Default::default()),
            whitespace_after_star: ParenthesizableWhitespace::SimpleWhitespace(Default::default()),
        }
    }
}

impl<'a> Param<'a> {
    fn codegen(
        &self,
        state: &mut CodegenState,
        default_star: Option<&str>,
        default_comma: bool,
    ) -> () {
        match (self.star, default_star) {
            (Some(star), _) => state.add_token(star.to_owned()),
            (None, Some(star)) => state.add_token(star.to_owned()),
            _ => {}
        }
        self.whitespace_after_star.codegen(state);
        self.name.codegen(state);

        // TODO: annotation here

        match (&self.equal, &self.default) {
            (Some(equal), Some(def)) => {
                equal.codegen(state);
                def.codegen(state);
            }
            (None, Some(def)) => {
                state.add_token(" = ".to_string());
                def.codegen(state);
            }
            _ => {}
        }

        match &self.comma {
            Some(comma) => comma.codegen(state),
            None if default_comma => state.add_token(", ".to_string()),
            _ => {}
        }

        self.whitespace_after_param.codegen(state);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct LeftParen<'a> {
    /// Any space that appears directly after this left parenthesis.
    whitespace_after: ParenthesizableWhitespace<'a>,
}

impl<'a> Codegen for LeftParen<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        state.add_token("(".to_string());
        self.whitespace_after.codegen(state);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RightParen<'a> {
    /// Any space that appears directly before this right parenthesis.
    whitespace_before: ParenthesizableWhitespace<'a>,
}

impl<'a> Codegen for RightParen<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        self.whitespace_before.codegen(state);
        state.add_token(")".to_string());
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Expression<'a> {
    Name(Name<'a>),
    Ellipsis {
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    Integer {
        /// A string representation of the integer, such as ``"100000"`` or
        /// ``"100_000"``.
        value: &'a str,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    Float {
        /// A string representation of the floating point number, such as ```"0.05"``,
        /// ``".050"``, or ``"5e-2"``.
        value: &'a str,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    Imaginary {
        /// A string representation of the complex number, such as ``"2j"``
        value: &'a str,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    SimpleString {
        /// The texual representation of the string, including quotes, prefix
        /// characters, and any escape characters present in the original source code,
        /// such as ``r"my string\n"``.
        value: &'a str,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    Comparison {
        left: Box<Expression<'a>>,
        comparisons: Vec<ComparisonTarget<'a>>,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    UnaryOperation {
        operator: &'a str, // TODO
        expression: Box<Expression<'a>>,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    BinaryOperation {
        left: Box<Expression<'a>>,
        operator: &'a str, // TODO
        right: Box<Expression<'a>>,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    BooleanOperation {
        left: Box<Expression<'a>>,
        operator: &'a str, // TODO
        right: Box<Expression<'a>>,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    Attribute {
        value: Box<Expression<'a>>,
        attr: Name<'a>,
        dot: &'a str, // TODO
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
    },
    NamedExpr {
        target: Box<Expression<'a>>,
        value: Box<Expression<'a>>,
        lpar: Vec<LeftParen<'a>>,
        rpar: Vec<RightParen<'a>>,
        whitespace_before_walrus: ParenthesizableWhitespace<'a>,
        whitespace_after_walrus: ParenthesizableWhitespace<'a>,
    }, // TODO: FormattedString, ConcatenatedString, Subscript, Lambda, Call, Await, IfExp, Yield, Tuple, List, Set, Dict, comprehensions
}

impl<'a> Codegen for Expression<'a> {
    fn codegen(&self, state: &mut CodegenState) -> () {
        match &self {
            &Self::Ellipsis { .. } => state.add_token("...".to_string()),
            &Self::BinaryOperation {
                left,
                operator,
                right,
                ..
            } => self.parenthesize(state, |state| {
                left.codegen(state);
                state.add_token(operator.to_string()); // TODO
                right.codegen(state);
            }),
            &Self::Integer { value, .. } => {
                self.parenthesize(state, |state| state.add_token(value.to_string()))
            }
            _ => panic!("codegen not implemented for {:#?}", self),
        }
    }
}

impl<'a> ParenthesizedNode<'a> for Expression<'a> {
    fn lpar(&self) -> &Vec<LeftParen<'a>> {
        match &self {
            &Self::BinaryOperation { lpar, .. } => lpar,
            &Self::Integer { lpar, .. } => lpar,
            _ => todo!(),
        }
    }

    fn rpar(&self) -> &Vec<RightParen<'a>> {
        match &self {
            &Self::BinaryOperation { rpar, .. } => rpar,
            &Self::Integer { rpar, .. } => rpar,
            _ => todo!(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ComparisonTarget<'a> {
    operator: &'a str, // TODO
    comparator: Expression<'a>,
}

trait ParenthesizedNode<'a> {
    fn lpar(&self) -> &Vec<LeftParen<'a>>;
    fn rpar(&self) -> &Vec<RightParen<'a>>;

    fn parenthesize<F>(&self, state: &mut CodegenState, f: F) -> ()
    where
        F: FnOnce(&mut CodegenState),
    {
        for lpar in self.lpar() {
            lpar.codegen(state);
        }
        f(state);
        for rpar in self.rpar() {
            rpar.codegen(state);
        }
    }
}