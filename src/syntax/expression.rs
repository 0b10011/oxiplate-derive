use super::{template::whitespace, Res};
use crate::syntax::item::tag_end;
use crate::{Source, State};
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_while, take_while1};
use nom::character::complete::char;
use nom::combinator::{cut, not, opt, peek};
use nom::error::context;
use nom::multi::{many0, many_till};
use nom::sequence::{pair, terminated, tuple};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};

// #[derive(Debug, PartialEq)]
// // https://doc.rust-lang.org/reference/expressions/literal-expr.html
// enum Literal<'a> {
//     Char(char),
//     String(&'a str),
//     Byte(u8),
//     ByteString(&'a Vec<u8>),
//     Integer(i64),
//     Float(f64),
//     Boolean(bool),
// }

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Keyword<'a>(pub Source<'a>);

impl ToTokens for Keyword<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let span = self.0.span();
        let keyword = syn::Ident::new(self.0.as_str(), span);
        tokens.append_all(quote_spanned! {span=> #keyword });
    }

    fn to_token_stream(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        self.to_tokens(&mut tokens);
        tokens
    }

    fn into_token_stream(self) -> TokenStream
    where
        Self: Sized,
    {
        self.to_token_stream()
    }
}

pub(super) fn keyword<'a>(
    keyword: &'static str,
) -> impl Fn(Source<'a>) -> Res<Source<'a>, Keyword<'a>> + 'a {
    move |input: Source<'a>| {
        let (input, keyword) = tag(keyword)(input)?;
        Ok((input, Keyword(keyword)))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Identifier<'a> {
    pub ident: &'a str,
    pub source: Source<'a>,
}

impl ToTokens for Identifier<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = match self.ident.to_ascii_lowercase().as_str() {
            keyword @ ("self" | "super") => panic!("{keyword} cannot be a raw identifier"),

            // Keywords from <https://doc.rust-lang.org/reference/keywords.html>.
            // Prefix with `r#` so Rust will accept them as idents.
            "abstract" | "as" | "async" | "await" | "become" | "box" | "break" | "const"
            | "continue" | "crate" | "do" | "dyn" | "else" | "enum" | "extern" | "false"
            | "final" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "macro"
            | "macro_rules" | "match" | "mod" | "move" | "mut" | "override" | "priv" | "pub"
            | "ref" | "return" | "static" | "struct" | "trait" | "true" | "try" | "type"
            | "typeof" | "union" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
            | "yield" => syn::Ident::new_raw(self.ident, self.source.span()),

            _ => syn::Ident::new(self.ident, self.source.span()),
        };

        tokens.append_all(quote! { #ident });
    }
}

pub(super) fn ident(input: Source) -> Res<Source, Identifier> {
    // Ignore if it starts with a number
    let (input, _) = peek(take_while1(
        |char: char| matches!(char, 'a'..='z' | 'A'..='Z' | '_'),
    ))(input)?;

    let (input, ident) = cut(take_while1(
        |char: char| matches!(char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'),
    ))(input)?;
    Ok((
        input,
        Identifier {
            ident: ident.as_str(),
            source: ident,
        },
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum IdentifierOrFunction<'a> {
    Identifier(Identifier<'a>),
    Function(Identifier<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum IdentifierScope {
    Local,
    Parent,
    Data,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IdentField<'a> {
    parents: Vec<Identifier<'a>>,
    ident_or_function: IdentifierOrFunction<'a>,
    scope: IdentifierScope,
}

impl<'a> IdentField<'a> {
    pub fn new(
        parents: Vec<Identifier<'a>>,
        ident_or_function: IdentifierOrFunction<'a>,
        scope: IdentifierScope,
    ) -> Self {
        Self {
            parents,
            ident_or_function,
            scope,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Expression<'a> {
    Identifier(IdentifierOrFunction<'a>, IdentifierScope),
    FieldAccess(IdentField<'a>),
    String(Source<'a>),
    Number(Source<'a>),
    Bool(bool, Source<'a>),
    // Group(Box<Expression<'a>>),
    Calc(Box<Expression<'a>>, Operator<'a>, Box<Expression<'a>>),
    Prefixed(PrefixOperator, Box<Expression<'a>>),
}

impl ToTokens for Expression<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            Expression::Identifier(identifier, scope) => match &identifier {
                IdentifierOrFunction::Identifier(identifier) => {
                    let span = identifier.source.span();
                    match scope {
                        IdentifierScope::Local => quote_spanned! {span=> #identifier },
                        IdentifierScope::Parent => quote_spanned! {span=> self.#identifier },
                        IdentifierScope::Data => quote_spanned! {span=> self._data.#identifier },
                    }
                }
                IdentifierOrFunction::Function(identifier) => {
                    let span = identifier.source.span();
                    match scope {
                        IdentifierScope::Local => quote_spanned! {span=> #identifier() },
                        IdentifierScope::Parent => quote_spanned! {span=> self.#identifier() },
                        IdentifierScope::Data => quote_spanned! {span=> self._data.#identifier() },
                    }
                }
            },
            Expression::FieldAccess(field) => {
                let parents = &field.parents;
                match &field.ident_or_function {
                    IdentifierOrFunction::Identifier(identifier) => {
                        let span = identifier.source.span();
                        for parent in parents {
                            span.join(parent.source.span());
                        }
                        match field.scope {
                            IdentifierScope::Local => {
                                quote_spanned! {span=> #(#parents.)*#identifier }
                            }
                            IdentifierScope::Parent => {
                                quote_spanned! {span=> self.#(#parents.)*#identifier }
                            }
                            IdentifierScope::Data => {
                                quote_spanned! {span=> self._data.#(#parents.)*#identifier }
                            }
                        }
                    }
                    IdentifierOrFunction::Function(identifier) => {
                        let span = identifier.source.span();
                        for parent in parents {
                            span.join(parent.source.span());
                        }
                        match field.scope {
                            IdentifierScope::Local => {
                                quote_spanned! {span=> #(#parents.)*#identifier() }
                            }
                            IdentifierScope::Parent => {
                                quote_spanned! {span=> self.#(#parents.)*#identifier() }
                            }
                            IdentifierScope::Data => {
                                quote_spanned! {span=> self._data.#(#parents.)*#identifier() }
                            }
                        }
                    }
                }
            }
            Expression::Calc(left, operator, right) => quote!(#left #operator #right),
            Expression::Prefixed(operator, expression) => quote!(#operator #expression),
            Expression::String(string) => {
                let string = ::syn::LitStr::new(string.as_str(), string.span());
                quote! {
                    #string
                }
            }
            Expression::Number(number) => {
                let number = ::syn::LitInt::new(number.as_str(), number.span());
                quote! {
                    #number
                }
            }
            Expression::Bool(bool, source) => {
                let bool = ::syn::LitBool::new(*bool, source.span());
                quote! { #bool }
            }
        });
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Operator<'a> {
    Addition(Source<'a>),
    Subtraction(Source<'a>),
    Multiplication(Source<'a>),
    Division(Source<'a>),
    Remainder(Source<'a>),

    Equal(Source<'a>),
    NotEqual(Source<'a>),
    GreaterThan(Source<'a>),
    LessThan(Source<'a>),
    GreaterThanOrEqual(Source<'a>),
    LessThanOrEqual(Source<'a>),

    Or(Source<'a>),
    And(Source<'a>),
}

impl ToTokens for Operator<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            Operator::Addition(source) => {
                let span = source.span();
                quote_spanned!(span=> +)
            }
            Operator::Subtraction(source) => {
                let span = source.span();
                quote_spanned!(span=> -)
            }
            Operator::Multiplication(source) => {
                let span = source.span();
                quote_spanned!(span=> *)
            }
            Operator::Division(source) => {
                let span = source.span();
                quote_spanned!(span=> /)
            }
            Operator::Remainder(source) => {
                let span = source.span();
                quote_spanned!(span=> %)
            }

            Operator::Equal(source) => {
                let span = source.span();
                quote_spanned!(span=> ==)
            }
            Operator::NotEqual(source) => {
                let span = source.span();
                quote_spanned!(span=> !=)
            }
            Operator::GreaterThan(source) => {
                let span = source.span();
                quote_spanned!(span=> >)
            }
            Operator::LessThan(source) => {
                let span = source.span();
                quote_spanned!(span=> <)
            }
            Operator::GreaterThanOrEqual(source) => {
                let span = source.span();
                quote_spanned!(span=> >=)
            }
            Operator::LessThanOrEqual(source) => {
                let span = source.span();
                quote_spanned!(span=> <=)
            }

            Operator::Or(source) => {
                let span = source.span();
                quote_spanned!(span=> ||)
            }
            Operator::And(source) => {
                let span = source.span();
                quote_spanned!(span=> &&)
            }
        });
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrefixOperator {
    Borrow,
    Dereference,
}

impl ToTokens for PrefixOperator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(match self {
            PrefixOperator::Borrow => quote!(&),
            PrefixOperator::Dereference => quote!(*),
        });
    }
}

pub(super) fn expression<'a>(state: &'a State) -> impl Fn(Source) -> Res<Source, Expression> + 'a {
    |input| {
        alt((
            string,
            number,
            bool,
            calc(state),
            field_or_identifier(state),
            prefixed_expression(state),
        ))(input)
    }
}

fn field_or_identifier<'a>(state: &'a State) -> impl Fn(Source) -> Res<Source, Expression> + 'a {
    |input| {
        let (input, (parsed_parents, (ident, maybe_function))) = pair(
            many0(terminated(&ident, char('.'))),
            pair(&ident, opt(tag("()"))),
        )(input)?;

        let ident_str = ident.ident;
        let field = if maybe_function.is_some() {
            IdentifierOrFunction::Function(ident)
        } else {
            IdentifierOrFunction::Identifier(ident)
        };
        let is_extending = input.original.is_extending;

        if parsed_parents.is_empty() {
            return Ok((
                input,
                Expression::Identifier(
                    field,
                    if state.local_variables.contains(ident_str) {
                        IdentifierScope::Local
                    } else if is_extending {
                        IdentifierScope::Data
                    } else {
                        IdentifierScope::Parent
                    },
                ),
            ));
        }

        let mut parents = Vec::with_capacity(parsed_parents.len());
        let mut is_local = None;
        for parent in parsed_parents {
            if is_local.is_none() {
                is_local = Some(state.local_variables.contains(parent.ident));
            }
            parents.push(parent);
        }

        Ok((
            input,
            Expression::FieldAccess(IdentField::new(
                parents,
                field,
                if is_local.unwrap_or(false) {
                    IdentifierScope::Local
                } else if is_extending {
                    IdentifierScope::Data
                } else {
                    IdentifierScope::Parent
                },
            )),
        ))
    }
}
fn operator(input: Source) -> Res<Source, Operator> {
    let (input, operator) = alt((
        tag("+"),
        tag("-"),
        tag("*"),
        tag("/"),
        tag("%"),
        tag("=="),
        tag("!="),
        tag(">="),
        tag("<="),
        tag(">"),
        tag("<"),
        tag("||"),
        tag("&&"),
    ))(input)?;

    let operator = match operator.as_str() {
        "+" => Operator::Addition(operator),
        "-" => Operator::Subtraction(operator),
        "*" => Operator::Multiplication(operator),
        "/" => Operator::Division(operator),
        "%" => Operator::Remainder(operator),

        "==" => Operator::Equal(operator),
        "!=" => Operator::NotEqual(operator),
        ">" => Operator::GreaterThan(operator),
        "<" => Operator::LessThan(operator),
        ">=" => Operator::GreaterThanOrEqual(operator),
        "<=" => Operator::LessThanOrEqual(operator),

        "||" => Operator::Or(operator),
        "&&" => Operator::And(operator),

        _ => unreachable!("All cases should be covered"),
    };

    Ok((input, operator))
}

/// Parses a bool value: `true` or `false`
fn bool(input: Source) -> Res<Source, Expression> {
    let (input, source) = alt((tag("true"), tag("false")))(input)?;
    let bool = match source.as_str() {
        "true" => true,
        "false" => false,
        _ => unreachable!("All cases should be covered"),
    };

    Ok((input, Expression::Bool(bool, source)))
}

fn number(input: Source) -> Res<Source, Expression> {
    // TODO: Add support for _ separatation
    // TODO: Add support for other number types? (e.g., 0b10011)
    // TODO: Fail on numbers like `0123`
    let (input, number) = take_while1(|char: char| char.is_ascii_digit())(input)?;
    Ok((input, Expression::Number(number)))
}
fn string(input: Source) -> Res<Source, Expression> {
    let (input, opening_hashes) = take_while(|c| c == '#')(input)?;

    let (input, _) = char('"')(input)?;

    let closing = pair(char('"'), tag(opening_hashes.as_str()));
    let (input, (string, _)) = many_till(take(1u32), closing)(input)?;
    let (input, _closing_hashes) = tag(opening_hashes.as_str())(input)?;

    let full_string = if let Some(full_string) = string.first() {
        let mut full_string = full_string.clone();
        full_string.range.end = string.last().unwrap().range.end;
        full_string
    } else {
        let mut full_string = opening_hashes.clone();
        full_string.range.start = full_string.range.end;
        full_string
    };
    Ok((input, Expression::String(full_string)))
}
fn calc<'a>(state: &'a State) -> impl Fn(Source) -> Res<Source, Expression> + 'a {
    |input| {
        let (input, (left, _leading_whitespace, (), operator, _trailing_whitespace, right)) =
            tuple((
                field_or_identifier(state),
                opt(whitespace),
                // End tags like `-}}` and `%}` could be matched by operator; this ensures we can use `cut()` later.
                not(alt((tag_end("}}"), tag_end("%}"), tag_end("#}")))),
                operator,
                opt(whitespace),
                context("Expected an expression", cut(expression(state))),
            ))(input)?;
        Ok((
            input,
            Expression::Calc(Box::new(left), operator, Box::new(right)),
        ))
    }
}
fn prefix_operator(input: Source) -> Res<Source, PrefixOperator> {
    let (input, operator) = alt((tag("&"), tag("*")))(input)?;
    let operator = match operator.as_str() {
        "&" => PrefixOperator::Borrow,
        "*" => PrefixOperator::Dereference,
        _ => unreachable!("All cases should be covered"),
    };

    Ok((input, operator))
}
fn prefixed_expression<'a>(state: &'a State) -> impl Fn(Source) -> Res<Source, Expression> + 'a {
    |input| {
        let (input, (prefix_operator, expression)) =
            tuple((prefix_operator, expression(state)))(input)?;

        Ok((
            input,
            Expression::Prefixed(prefix_operator, Box::new(expression)),
        ))
    }
}
