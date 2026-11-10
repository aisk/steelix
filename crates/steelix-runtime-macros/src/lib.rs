//! The attribute macros of `steelix-runtime`. Use them through that crate
//! (`rt::frame`), which documents them.
//!
//! TODO: move `frame` into `steelix-runtime` as a declarative attribute
//! macro (`macro_rules! frame { attr(..) (..) => .. }`) and drop this crate
//! once `macro_attr` is stable (rust-lang/rust#143547). A dozen lines of
//! `macro_rules!` that collect the tokens up to the body do the same, and
//! the generated code does not change: macro hygiene is the reason why the
//! attribute names the frame variable instead of making one up.

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

/// `#[rt::frame(scope, "qualname", line)]` on a `fn` or `async fn`: runs its
/// body in a traceback frame. Two more arguments are optional, in this
/// order: the name of the frame for the body (`frame`) and `recursive`. See
/// `steelix_runtime::frame`.
#[proc_macro_attribute]
pub fn frame(args: TokenStream, item: TokenStream) -> TokenStream {
    expand(args, item).unwrap_or_else(|(span, message)| compile_error(span, message))
}

type Error = (Span, &'static str);

const USAGE: &str = "expected `#[rt::frame(scope, \"qualname\", line)]`, \
                     optionally followed by the name of the frame and `recursive`";

fn expand(args: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
    let mut args = split_arguments(args);
    if !(3..=5).contains(&args.len()) || args.iter().any(Vec::is_empty) {
        return Err((Span::call_site(), USAGE));
    }
    // After the line: the name the body uses for the frame (`_` without
    // one), then `recursive` for a function that checks the stack.
    let mut frame: TokenTree = Ident::new("_", Span::call_site()).into();
    let mut recursive = false;
    for option in args.split_off(3) {
        match single(option) {
            Some(TokenTree::Ident(word)) if word.to_string() == "recursive" && !recursive => {
                recursive = true;
            }
            Some(name @ TokenTree::Ident(_)) if !recursive && frame.to_string() == "_" => {
                frame = name;
            }
            _ => return Err((Span::call_site(), USAGE)),
        }
    }
    let [scope, qualname, line] = <[Vec<TokenTree>; 3]>::try_from(args).expect("three arguments");
    let (Some(TokenTree::Literal(qualname)), Some(TokenTree::Literal(line))) =
        (single(qualname), single(line))
    else {
        return Err((Span::call_site(), USAGE));
    };
    let scope: TokenStream = scope.into_iter().collect();

    let mut signature: Vec<TokenTree> = item.into_iter().collect();
    let body = match signature.pop() {
        Some(TokenTree::Group(body)) if body.delimiter() == Delimiter::Brace => body,
        _ => return Err((Span::call_site(), "`rt::frame` goes on a function")),
    };
    // The qualifiers before `fn`, after any attributes and doc comments.
    let is_async = signature
        .iter()
        .map(|token| match token {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => String::new(),
        })
        .take_while(|word| word != "fn")
        .any(|word| word == "async");

    // `static CODE` in a block of its own: the body does not see the name.
    let mut code = tokens("static CODE: ::steelix_runtime::frame::Code =");
    code.extend(tokens("::steelix_runtime::frame::Code::new"));
    let mut code_args = TokenStream::new();
    code_args.extend([
        TokenTree::Literal(qualname),
        Punct::new(',', Spacing::Alone).into(),
        TokenTree::Literal(line),
    ]);
    code.extend([TokenTree::Group(Group::new(
        Delimiter::Parenthesis,
        code_args,
    ))]);
    if recursive {
        code.extend(tokens(".recursive()"));
    }
    code.extend(tokens("; &CODE"));

    // `run(scope, { .. }, |frame| { body })`, or for an `async fn`
    // `run_async(&scope, { .. }, async |frame| { body }).await`.
    let mut run_args = TokenStream::new();
    if is_async {
        run_args.extend(tokens("&"));
    }
    run_args.extend(scope);
    run_args.extend(tokens(","));
    run_args.extend([TokenTree::Group(Group::new(Delimiter::Brace, code))]);
    run_args.extend(tokens(if is_async { ", async |" } else { ", |" }));
    run_args.extend([frame]);
    run_args.extend(tokens("|"));
    run_args.extend([TokenTree::Group(body.clone())]);

    let mut call = tokens(if is_async {
        "::steelix_runtime::frame::run_async"
    } else {
        "::steelix_runtime::frame::run"
    });
    call.extend([TokenTree::Group(Group::new(
        Delimiter::Parenthesis,
        run_args,
    ))]);
    if is_async {
        call.extend(tokens(".await"));
    }
    let mut block = Group::new(Delimiter::Brace, call);
    block.set_span(body.span());

    let mut function: TokenStream = signature.into_iter().collect();
    function.extend([TokenTree::Group(block)]);
    Ok(function)
}

/// The arguments of the attribute, split at the commas.
fn split_arguments(args: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut parts = vec![Vec::new()];
    for token in args {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => parts.push(Vec::new()),
            _ => parts.last_mut().expect("never empty").push(token),
        }
    }
    // A trailing comma.
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    parts
}

fn single(mut part: Vec<TokenTree>) -> Option<TokenTree> {
    match part.len() {
        1 => part.pop(),
        _ => None,
    }
}

fn tokens(source: &str) -> TokenStream {
    source.parse().expect("valid Rust tokens")
}

fn compile_error(span: Span, message: &str) -> TokenStream {
    let mut error: TokenStream = tokens("::core::compile_error!");
    let literal = TokenTree::Literal(proc_macro::Literal::string(message));
    let mut group = Group::new(Delimiter::Parenthesis, TokenStream::from(literal));
    group.set_span(span);
    error.extend([TokenTree::Group(group)]);
    error.extend(tokens(";"));
    error
}
