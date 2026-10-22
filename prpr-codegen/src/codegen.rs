//! The Rust back end.
//!
//! Every expression is compiled to a Rust expression that matches at `__pos`. In value mode it
//! evaluates to `Result<(T, usize), Fail>`, in recognition mode to `Result<usize, Fail>`.
//! Generated local names start with `__` and use mixed-site hygiene.

use crate::analysis::{Analysis, Boundary, InputKind, RuleInfo, is_literal_choice};
use crate::ast::*;
use crate::diag::Diagnostic;
use crate::source::Source;
use proc_macro2::{
    Group, Ident, Literal, Punct, Spacing, Span as TokenSpan, TokenStream, TokenTree,
};
use quote::quote_spanned;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;

macro_rules! q {
    ($($t:tt)*) => {
        quote_spanned!(TokenSpan::mixed_site()=> $($t)*)
    };
}

/// Options of code generation.
#[derive(Clone, Debug)]
pub struct Options {
    /// The path of the runtime in generated code, e.g. `::prpr::runtime`.
    pub runtime: TokenStream,
    /// Whether generated code checks at compile time that the runtime is compatible.
    pub check_version: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Value,
    Recognize,
}

#[derive(Clone, Copy)]
struct Ctx {
    // Whether implicit whitespace is skipped before tokens.
    skipping: bool,
}

// The delimiter check after a token.
enum Check {
    None,
    // The token ends with a word character, so no word character may follow.
    NoWordAfter,
    // Decided at run time from the matched text.
    Delimited,
    // `#[boundary(e)]` of the rule.
    Custom(Ident),
}

struct Gen<'a> {
    src: &'a dyn Source,
    an: &'a Analysis,
    rt: TokenStream,
    input: TokenStream,
    labels: Cell<usize>,
    diags: RefCell<Vec<Diagnostic>>,
}

pub fn generate(
    src: &dyn Source,
    an: &Analysis,
    opts: &Options,
) -> Result<TokenStream, Vec<Diagnostic>> {
    let input = match an.input {
        InputKind::Str => q!(str),
        InputKind::Bytes => q!([u8]),
    };
    let generator = Gen {
        src,
        an,
        rt: opts.runtime.clone(),
        input,
        labels: Cell::new(0),
        diags: RefCell::new(Vec::new()),
    };
    let mut out = TokenStream::new();
    if opts.check_version {
        let rt = &generator.rt;
        let (major, minor) = prpr_runtime::VERSION;
        let (major, minor) = (
            Literal::u32_unsuffixed(major),
            Literal::u32_unsuffixed(minor),
        );
        out.extend(q!(const _: () = #rt::require_version(#major, #minor);));
    }
    out.extend(generator.grammar());
    let diags = generator.diags.into_inner();
    if diags.iter().any(Diagnostic::is_error) {
        Err(diags)
    } else {
        Ok(out)
    }
}

fn ident(name: &str) -> Ident {
    Ident::new(name, TokenSpan::mixed_site())
}

fn raw_name(name: &str) -> Ident {
    Ident::new(&format!("__rule_{name}"), TokenSpan::call_site())
}

fn boundary_name(name: &str) -> Ident {
    Ident::new(&format!("__boundary_{name}"), TokenSpan::call_site())
}

fn memo_field(name: &str) -> Ident {
    Ident::new(&format!("__memo_{name}"), TokenSpan::call_site())
}

fn clone_fn(name: &str) -> Ident {
    Ident::new(&format!("__clone_{name}"), TokenSpan::call_site())
}

fn atom_const(name: &str) -> Ident {
    Ident::new(&format!("__atom_{name}"), TokenSpan::call_site())
}

fn delimited_fn(name: &str) -> Ident {
    Ident::new(&format!("__delimited_{name}"), TokenSpan::call_site())
}

fn text_var(name: &str) -> Ident {
    ident(&format!("__text_{name}"))
}

fn span_var(name: &str) -> Ident {
    ident(&format!("__span_{name}"))
}

fn start_var(name: &str) -> Ident {
    ident(&format!("__start_{name}"))
}

// The path of the module of the grammar that a reference to another grammar refers to.
fn module_of(path: &Path) -> Path {
    Path {
        leading_colon: path.leading_colon,
        segments: path.segments[..path.segments.len() - 1].to_vec(),
        span: path.span,
    }
}

// The field of `Memo` for the state of another grammar.
fn grammar_field(module: &Path) -> Ident {
    let names: Vec<&str> = module.segments.iter().map(|s| s.name.as_str()).collect();
    let root = if module.leading_colon { "_" } else { "" };
    Ident::new(
        &format!("__g_{root}{}", names.join("__")),
        TokenSpan::call_site(),
    )
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.leading_colon == b.leading_colon
        && a.segments.len() == b.segments.len()
        && a.segments
            .iter()
            .zip(&b.segments)
            .all(|(x, y)| x.name == y.name)
}

fn class_contains(class: &Class, c: char) -> bool {
    class.ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi) != class.negated
}

impl Gen<'_> {
    fn error(&self, d: Diagnostic) {
        self.diags.borrow_mut().push(d);
    }

    fn tokens(&self, span: Span) -> TokenStream {
        self.src.tokens(span).unwrap_or_else(|d| {
            self.error(d);
            TokenStream::new()
        })
    }

    // A fresh label for a block.
    fn label(&self) -> TokenStream {
        let n = self.labels.get();
        self.labels.set(n + 1);
        [
            TokenTree::Punct(Punct::new('\'', Spacing::Joint)),
            TokenTree::Ident(ident(&format!("__l{n}"))),
        ]
        .into_iter()
        .collect()
    }

    fn user_ident(&self, id: &crate::ast::Ident) -> Ident {
        Ident::new(&id.name, self.src.token_span(id.span))
    }

    fn grammar(&self) -> TokenStream {
        let mut out = TokenStream::new();
        for &span in &self.an.rust_items {
            out.extend(self.tokens(span));
        }
        out.extend(self.memo_struct());
        out.extend(self.helpers());
        for info in &self.an.rules {
            out.extend(self.rule(info));
        }
        out.extend(self.rules_module());
        out
    }

    fn memo_struct(&self) -> TokenStream {
        let rt = &self.rt;
        let memo_rules: Vec<&RuleInfo> = self.an.rules.iter().filter(|info| info.memo).collect();
        let tables = memo_rules.iter().map(|info| {
            let field = memo_field(&info.rule.name.name);
            let ty = self.rule_type(info);
            q!(#field: #rt::MemoTable<#ty>,)
        });
        // The only place that requires the values to be `Clone`, so that a missing
        // implementation is reported once, at the type of the rule.
        let clones = memo_rules.iter().map(|info| {
            let name = clone_fn(&info.rule.name.name);
            let ty = self.rule_type(info);
            let span = info
                .rule
                .ty
                .map_or(TokenSpan::call_site(), |t| self.src.token_span(t));
            let clone = quote_spanned!(span=> <#ty as ::core::clone::Clone>::clone);
            q! {
                #[inline]
                fn #name(__v: &#ty) -> #ty {
                    #clone(__v)
                }
            }
        });
        let grammars = self.other_grammars().into_iter().map(|module| {
            let field = grammar_field(&module);
            let module = self.path_tokens(&module);
            q!(#field: #module::Memo<'i>,)
        });
        q! {
            /// The state of a parse with this grammar besides the input.
            #[allow(dead_code, non_snake_case)]
            #[derive(Default)]
            pub struct Memo<'i> {
                __skip: #rt::SkipCache,
                #(#tables)*
                #(#grammars)*
                __phantom: ::core::marker::PhantomData<&'i ()>,
            }

            #[allow(dead_code, non_snake_case)]
            impl<'i> Memo<'i> {
                #(#clones)*
            }
        }
    }

    // The paths of the modules of other grammars whose rules are referenced.
    fn other_grammars(&self) -> Vec<Path> {
        fn walk(e: &Expr, out: &mut Vec<Path>) {
            if let ExprKind::Ref(path) = &e.kind
                && path.as_local().is_none()
            {
                let module = module_of(path);
                if !out.iter().any(|m| same_path(m, &module)) {
                    out.push(module);
                }
            }
            e.for_each_child(|c| walk(c, out));
        }
        let mut out = Vec::new();
        for info in &self.an.rules {
            walk(&info.rule.body, &mut out);
            if let Boundary::Custom(e) = &info.boundary {
                walk(e, &mut out);
            }
        }
        out
    }

    fn path_tokens(&self, path: &Path) -> TokenStream {
        let segments = path.segments.iter().map(|s| self.user_ident(s));
        let colon = path.leading_colon.then(|| q!(::));
        q!(#colon #(#segments)::*)
    }

    fn rule_type(&self, info: &RuleInfo) -> TokenStream {
        match info.rule.ty {
            Some(ty) if info.typed => self.tokens(ty),
            _ => q!(()),
        }
    }

    fn helpers(&self) -> TokenStream {
        let rt = &self.rt;
        let input = &self.input;
        let spaces = raw_name(crate::analysis::SPACES);
        let word_char = raw_name(crate::analysis::WORD_CHAR);
        let mut out = q! {
            // Skips implicit whitespace.
            #[allow(dead_code)]
            #[inline]
            fn __skip<'i>(__cx: &mut #rt::Cx<'i, #input>, __m: &mut Memo<'i>, __pos: usize) -> usize {
                if let ::core::option::Option::Some(__end) = __m.__skip.get(__pos) {
                    return __end;
                }
                __cx.push_quiet();
                let __end = match #spaces(__cx, __m, __pos) {
                    Ok((_, __p)) => __p,
                    Err(_) => __pos,
                };
                __cx.pop_quiet();
                __m.__skip.set(__pos, __end);
                __end
            }

            // The end of the word character at `__pos`, if there is one.
            #[allow(dead_code)]
            #[inline]
            fn __word_char<'i>(
                __cx: &mut #rt::Cx<'i, #input>,
                __m: &mut Memo<'i>,
                __pos: usize,
            ) -> ::core::option::Option<usize> {
                __cx.push_quiet();
                let __r = #word_char(__cx, __m, __pos);
                __cx.pop_quiet();
                match __r {
                    Ok((_, __p)) => ::core::option::Option::Some(__p),
                    Err(_) => ::core::option::Option::None,
                }
            }

            // Whether the token from `__start` to `__end` is delimited, i.e. it does not end with a
            // word character that is followed by another one.
            #[allow(dead_code)]
            fn __delimited<'i>(
                __cx: &mut #rt::Cx<'i, #input>,
                __m: &mut Memo<'i>,
                __start: usize,
                __end: usize,
            ) -> bool {
                if __end <= __start {
                    return true;
                }
                let ::core::option::Option::Some(__last) = __cx.char_start_before(__end) else {
                    return true;
                };
                __word_char(__cx, __m, __last) != ::core::option::Option::Some(__end)
                    || __word_char(__cx, __m, __end).is_none()
            }
        };
        for info in &self.an.rules {
            if let Boundary::Custom(e) = &info.boundary {
                let name = boundary_name(&info.rule.name.name);
                let body = self.expr(e, Ctx { skipping: false }, Mode::Recognize, None);
                out.extend(q! {
                    #[allow(unused, clippy::all)]
                    fn #name<'i>(__cx: &mut #rt::Cx<'i, #input>, __m: &mut Memo<'i>, __pos: usize) -> bool {
                        __cx.push_quiet();
                        let __r = #body;
                        __cx.pop_quiet();
                        __r.is_ok()
                    }
                });
            }
        }
        out
    }

    fn rule(&self, info: &RuleInfo) -> TokenStream {
        let rt = &self.rt;
        let input = &self.input;
        let rule = &info.rule;
        let raw = raw_name(&rule.name.name);
        let cx = Ctx {
            skipping: !rule.atom,
        };
        let ty = self.rule_type(info);
        let body = if info.typed {
            self.expr(&rule.body, cx, Mode::Value, Some(&ty))
        } else {
            let body = self.expr(&rule.body, cx, Mode::Recognize, None);
            q!(match #body { Ok(__p) => Ok(((), __p)), Err(__f) => Err(__f) })
        };
        let body = match &info.label {
            Some(label) => {
                let at = if cx.skipping {
                    q!(__skip(__cx, __m, __pos))
                } else {
                    q!(__pos)
                };
                q! {
                    let __at = #at;
                    let __saved = __cx.push_label(__at, #label);
                    let __r = #body;
                    __cx.pop_label(__saved);
                    __r
                }
            }
            None => body,
        };
        // Whitespace is never reported as expected, however it is invoked.
        let body = if crate::analysis::QUIET_RULES.contains(&rule.name.name.as_str()) {
            q! {
                __cx.push_quiet();
                let __r = { #body };
                __cx.pop_quiet();
                __r
            }
        } else {
            body
        };
        let body = if info.memo {
            let field = memo_field(&rule.name.name);
            let clone = clone_fn(&rule.name.name);
            q! {
                if let ::core::option::Option::Some(__r) = __m.#field.get(__pos, Memo::#clone) {
                    return __r;
                }
                let __r = { #body };
                __m.#field.insert(__pos, &__r, Memo::#clone);
                __r
            }
        } else {
            body
        };
        let vis = rule.vis.map(|v| self.tokens(v)).unwrap_or_default();
        let raw_fn = q! {
            #[doc(hidden)]
            #[allow(unused, non_snake_case, clippy::all)]
            #vis fn #raw<'i>(
                __cx: &mut #rt::Cx<'i, #input>,
                __m: &mut Memo<'i>,
                __pos: usize,
            ) -> #rt::PResult<#ty> {
                #body
            }
        };
        if info.builtin {
            return raw_fn;
        }

        let name = self.user_ident(&rule.name);
        let docs = if info.docs.is_empty() {
            vec![format!(
                " Parses the whole input with the rule `{}`.",
                rule.name.name
            )]
        } else {
            info.docs.clone()
        };
        let trailing = if rule.atom {
            q!(#rt::no_skip)
        } else {
            q!(__skip)
        };
        q! {
            #raw_fn

            #(#[doc = #docs])*
            #[allow(dead_code)]
            #vis fn #name<'i>(input: &'i #input) -> ::core::result::Result<#ty, #rt::ParseError> {
                #rt::parse_full(input, #raw, #trailing)
            }
        }
    }

    fn rules_module(&self) -> TokenStream {
        let rt = &self.rt;
        let input = &self.input;
        let mut delimited_fns = TokenStream::new();
        let uses = self.an.rules.iter().filter(|r| !r.builtin).map(|info| {
            let rule = &info.rule;
            let raw = raw_name(&rule.name.name);
            let name = Ident::new(&rule.name.name, TokenSpan::call_site());
            let vis = self.child_vis(rule.vis);
            let atom = rule.atom;
            let atom_const = atom_const(&rule.name.name);
            let delimited = delimited_fn(&rule.name.name);
            let check = match (&info.boundary, atom) {
                (_, false) | (Boundary::None, _) => q!(true),
                (Boundary::Default, true) => q!(__delimited(__cx, __m, __start, __end)),
                (Boundary::Custom(_), true) => {
                    let f = boundary_name(&rule.name.name);
                    q!(#f(__cx, __m, __end))
                }
            };
            let fn_vis = rule.vis.map(|v| self.tokens(v)).unwrap_or_default();
            delimited_fns.extend(q! {
                #[doc(hidden)]
                #[allow(unused, non_snake_case)]
                #fn_vis fn #delimited<'i>(
                    __cx: &mut #rt::Cx<'i, #input>,
                    __m: &mut Memo<'i>,
                    __start: usize,
                    __end: usize,
                ) -> bool {
                    #check
                }
            });
            q! {
                #vis use super::#raw as #name;
                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                #vis const #atom_const: bool = #atom;
                #[doc(hidden)]
                #vis use super::#delimited;
            }
        });
        let uses: Vec<TokenStream> = uses.collect();
        q! {
            #delimited_fns

            /// The rules of the grammar as functions that can be called from other parsers.
            #[allow(unused_imports)]
            pub mod rules {
                #(#uses)*
            }
        }
    }

    // The visibility in the child module `rules` that is equivalent to `vis` in the grammar's
    // module.
    fn child_vis(&self, vis: Option<Span>) -> TokenStream {
        let Some(span) = vis else {
            return q!(pub(super));
        };
        let text: String = self.src.text()[span]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        match text.as_str() {
            "pub" => q!(pub),
            "pub(crate)" => q!(pub(crate)),
            "pub(self)" => q!(pub(super)),
            "pub(super)" => q!(pub(in super::super)),
            _ => match text
                .strip_prefix("pub(in")
                .and_then(|p| p.strip_suffix(')'))
            {
                Some(path) if path.starts_with("crate") || path.starts_with("::") => {
                    self.tokens(span)
                }
                Some(path) => {
                    let path: TokenStream = path.parse().unwrap_or_default();
                    q!(pub(in super::#path))
                }
                None => self.tokens(span),
            },
        }
    }

    fn expr(&self, e: &Expr, cx: Ctx, mode: Mode, ret: Option<&TokenStream>) -> TokenStream {
        match &e.kind {
            ExprKind::Lit(lit) => self.lit(lit, cx, mode),
            // A single character is not a word, so it needs no delimiter.
            ExprKind::Class(class) => {
                let code = self.class(class, mode);
                self.token(cx, code, mode, Check::None, false)
            }
            ExprKind::Any => {
                let code = self.any(mode);
                self.token(cx, code, mode, Check::None, false)
            }
            ExprKind::Ref(path) => self.reference(path, cx, mode),
            ExprKind::Seq(elems) => self.seq(elems, None, cx, mode, ret),
            ExprKind::Action { expr, code, pred } => {
                let elems = match &expr.kind {
                    ExprKind::Seq(elems) => elems.as_slice(),
                    _ => std::slice::from_ref(&**expr),
                };
                self.seq(elems, Some((*code, *pred)), cx, mode, ret)
            }
            ExprKind::Bind { expr, .. } => self.expr(expr, cx, mode, ret),
            ExprKind::Choice(alts) => self.choice(alts, cx, mode, ret),
            ExprKind::Atomic(inner) => {
                let code = self.expr(inner, Ctx { skipping: false }, mode, ret);
                self.token(cx, code, mode, Check::Delimited, true)
            }
            ExprKind::And(inner) => {
                let code = self.expr(inner, cx, Mode::Recognize, None);
                let ok = unit_ok(mode);
                q!(match #code { Ok(_) => #ok, Err(__f) => Err(__f) })
            }
            ExprKind::Not(inner) => {
                let rt = &self.rt;
                let code = self.expr(inner, cx, Mode::Recognize, None);
                let ok = unit_ok(mode);
                let at = self.after_skip(cx);
                let fail = if matches!(inner.kind, ExprKind::Any) {
                    q!(__cx.fail(__at, #rt::Expected::Eof))
                } else {
                    q!(__cx.fail_unexpected(__at))
                };
                q! {{
                    __cx.push_quiet();
                    let __r = #code;
                    __cx.pop_quiet();
                    match __r {
                        Ok(_) => {
                            let __at = #at;
                            Err(#fail)
                        }
                        Err(_) => #ok,
                    }
                }}
            }
            ExprKind::Opt(inner) => {
                let code = self.expr(inner, cx, mode, None);
                match mode {
                    Mode::Value => q! {
                        match #code {
                            Ok((__v, __p)) => Ok((::core::option::Option::Some(__v), __p)),
                            Err(_) => Ok((::core::option::Option::None, __pos)),
                        }
                    },
                    Mode::Recognize => q! {
                        match #code {
                            Ok(__p) => Ok(__p),
                            Err(_) => Ok(__pos),
                        }
                    },
                }
            }
            ExprKind::Many(inner) => self.repeat(inner, None, false, cx, mode),
            ExprKind::Many1(inner) => self.repeat(inner, None, true, cx, mode),
            ExprKind::SepBy { expr, sep, min1 } => self.repeat(expr, Some(sep), *min1, cx, mode),
        }
    }

    fn after_skip(&self, cx: Ctx) -> TokenStream {
        if cx.skipping {
            q!(__skip(__cx, __m, __pos))
        } else {
            q!(__pos)
        }
    }

    // Skips whitespace before a token and checks the delimiter after it.
    //
    // An `opaque` token (a reference to an atom or an atomic group) is treated like a token from
    // a lexer: when it matches, the failures inside it are forgotten, so that errors after it
    // are about what may follow the token rather than about longer tokens.
    fn token(
        &self,
        cx: Ctx,
        code: TokenStream,
        mode: Mode,
        check: Check,
        opaque: bool,
    ) -> TokenStream {
        if !cx.skipping {
            return code;
        }
        let rt = &self.rt;
        let end = match mode {
            Mode::Value => q!(__r.1),
            Mode::Recognize => q!(__r),
        };
        let delimited = match check {
            Check::None => None,
            Check::NoWordAfter => Some(q!(__word_char(__cx, __m, #end).is_none())),
            Check::Delimited => Some(q!(__delimited(__cx, __m, __pos, #end))),
            Check::Custom(f) => Some(q!(#f(__cx, __m, #end))),
        };
        let matched = match delimited {
            Some(delimited) => q! {
                if #delimited {
                    Ok(__r)
                } else {
                    Err(__cx.fail(#end, #rt::Expected::Delimiter))
                }
            },
            None => q!(Ok(__r)),
        };
        if opaque {
            q! {{
                let __pos = __skip(__cx, __m, __pos);
                let __mark = __cx.mark();
                match #code {
                    Ok(__r) => {
                        __cx.rollback(__mark);
                        #matched
                    }
                    Err(__f) => {
                        __cx.commit(__mark);
                        Err(__f)
                    }
                }
            }}
        } else {
            q! {{
                let __pos = __skip(__cx, __m, __pos);
                match #code {
                    Ok(__r) => #matched,
                    Err(__f) => Err(__f),
                }
            }}
        }
    }

    fn lit(&self, lit: &Lit, cx: Ctx, mode: Mode) -> TokenStream {
        let (code, value, last) = match lit {
            Lit::Str(s) => {
                let l = Literal::string(s);
                (q!(__cx.lit(__pos, #l)), q!(#l), s.chars().last())
            }
            Lit::Bytes(b) => {
                let l = Literal::byte_string(b);
                (
                    q!(__cx.lit_bytes(__pos, #l)),
                    q!(#l),
                    b.last().map(|&b| char::from(b)),
                )
            }
        };
        let code = match mode {
            Mode::Value => q!(match #code { Ok(__p) => Ok((#value, __p)), Err(__f) => Err(__f) }),
            Mode::Recognize => code,
        };
        let check = match (last, &self.an.word_class) {
            (None, _) => Check::None,
            (Some(c), Some(class)) if class_contains(class, c) => Check::NoWordAfter,
            (Some(_), Some(_)) => Check::None,
            (Some(_), None) => Check::Delimited,
        };
        self.token(cx, code, mode, check, false)
    }

    fn class(&self, class: &Class, mode: Mode) -> TokenStream {
        let rt = &self.rt;
        let text = Literal::string(&class.text);
        let test = class_test(class, q!(__c), |c| match self.an.input {
            InputKind::Str => Literal::character(c),
            InputKind::Bytes => Literal::u8_unsuffixed(c as u8),
        });
        let test = if class.negated { q!(!#test) } else { test };
        let peek = self.peek();
        let ok = match mode {
            Mode::Value => q!(Ok((__c, __p))),
            Mode::Recognize => q!(Ok(__p)),
        };
        q! {
            match __cx.#peek(__pos) {
                ::core::option::Option::Some((__c, __p)) if #test => #ok,
                _ => Err(__cx.fail(__pos, #rt::Expected::Class(#text))),
            }
        }
    }

    // A test of a byte `__b` that is equivalent to matching `class` at the byte, if there is one:
    // with text input, for classes of ASCII characters, since ASCII bytes are never part of other
    // characters. Bytes of other characters are in a negated class.
    fn byte_test(&self, class: &Class) -> Option<TokenStream> {
        let byte_input = self.an.input == InputKind::Bytes;
        if !byte_input && class.ranges.iter().any(|&(_, hi)| !hi.is_ascii()) {
            return None;
        }
        let test = class_test(class, q!(__b), |c| Literal::u8_suffixed(c as u8));
        Some(match (class.negated, byte_input) {
            (false, _) => test,
            (true, true) => q!(!#test),
            (true, false) => q!(__b >= 0x80 || !#test),
        })
    }

    // `class*` or `class+` that is only recognized, as a loop over bytes.
    fn class_run(&self, class: &Class, test: TokenStream, min1: bool) -> TokenStream {
        let rt = &self.rt;
        let text = Literal::string(&class.text);
        let finish = if min1 {
            q! {
                if __p == __pos {
                    Err(__cx.fail(__pos, #rt::Expected::Class(#text)))
                } else {
                    __cx.expect(__p, #rt::Expected::Class(#text));
                    Ok(__p)
                }
            }
        } else {
            q! {
                __cx.expect(__p, #rt::Expected::Class(#text));
                Ok(__p)
            }
        };
        q! {{
            let __bytes = __cx.bytes();
            let mut __p = __pos;
            while let ::core::option::Option::Some(&__b) = __bytes.get(__p) {
                if !(#test) {
                    break;
                }
                __p += 1;
            }
            #finish
        }}
    }

    fn any(&self, mode: Mode) -> TokenStream {
        let rt = &self.rt;
        let peek = self.peek();
        let ok = match mode {
            Mode::Value => q!(Ok((__c, __p))),
            Mode::Recognize => q!(Ok(__p)),
        };
        q! {
            match __cx.#peek(__pos) {
                ::core::option::Option::Some((__c, __p)) => #ok,
                ::core::option::Option::None => Err(__cx.fail(__pos, #rt::Expected::Any)),
            }
        }
    }

    fn peek(&self) -> TokenStream {
        match self.an.input {
            InputKind::Str => q!(peek_char),
            InputKind::Bytes => q!(peek_byte),
        }
    }

    fn reference(&self, path: &Path, cx: Ctx, mode: Mode) -> TokenStream {
        let Some(info) = self.an.resolve(path) else {
            return self.other_reference(path, cx, mode);
        };
        let raw = raw_name(&info.rule.name.name);
        let call = q!(#raw(__cx, __m, __pos));
        let code = match mode {
            Mode::Value => call,
            Mode::Recognize => q!(match #call { Ok((_, __p)) => Ok(__p), Err(__f) => Err(__f) }),
        };
        if !info.rule.atom {
            return code;
        }
        let check = match &info.boundary {
            Boundary::Default => Check::Delimited,
            Boundary::None => Check::None,
            Boundary::Custom(_) => Check::Custom(boundary_name(&info.rule.name.name)),
        };
        self.token(cx, code, mode, check, true)
    }

    // A rule of another grammar. Whether it is a token is known from constants of that grammar.
    fn other_reference(&self, path: &Path, cx: Ctx, mode: Mode) -> TokenStream {
        let rt = &self.rt;
        let name = path.segments.last().unwrap();
        let module = module_of(path);
        let field = grammar_field(&module);
        let module = self.path_tokens(&module);
        let rule = self.user_ident(name);
        let atom = atom_const(&name.name);
        let delimited = delimited_fn(&name.name);
        let call = q!(#module::rules::#rule(__cx, &mut __m.#field, __pos));
        let code = match mode {
            Mode::Value => call,
            Mode::Recognize => q!(match #call { Ok((_, __p)) => Ok(__p), Err(__f) => Err(__f) }),
        };
        if !cx.skipping {
            return code;
        }
        let end = match mode {
            Mode::Value => q!(__r.1),
            Mode::Recognize => q!(__r),
        };
        q! {{
            const __ATOM: bool = #module::rules::#atom;
            let __pos = if __ATOM { __skip(__cx, __m, __pos) } else { __pos };
            let __mark = if __ATOM { ::core::option::Option::Some(__cx.mark()) } else { ::core::option::Option::None };
            match #code {
                Ok(__r) => {
                    if let ::core::option::Option::Some(__mark) = __mark {
                        __cx.rollback(__mark);
                    }
                    if !__ATOM || #module::rules::#delimited(__cx, &mut __m.#field, __pos, #end) {
                        Ok(__r)
                    } else {
                        Err(__cx.fail(#end, #rt::Expected::Delimiter))
                    }
                }
                Err(__f) => {
                    if let ::core::option::Option::Some(__mark) = __mark {
                        __cx.commit(__mark);
                    }
                    Err(__f)
                }
            }
        }}
    }

    fn choice(&self, alts: &[Expr], cx: Ctx, mode: Mode, ret: Option<&TokenStream>) -> TokenStream {
        // A choice between silent alternatives has the value `()`.
        if mode == Mode::Value && !is_literal_choice(alts) && alts.iter().all(|e| self.an.silent(e))
        {
            let code = self.choice(alts, cx, Mode::Recognize, None);
            return q!(match #code { Ok(__p) => Ok(((), __p)), Err(__f) => Err(__f) });
        }
        let label = self.label();
        let (last, init) = alts.split_last().expect("a choice has alternatives");
        let init = init.iter().map(|alt| {
            let code = self.expr(alt, cx, mode, ret);
            q!(if let Ok(__r) = #code { break #label Ok(__r); })
        });
        let last = self.expr(last, cx, mode, ret);
        q!(#label: { #(#init)* #last })
    }

    // `e*`, `e+`, `(e, sep)*` and `(e, sep)+`.
    fn repeat(&self, e: &Expr, sep: Option<&Expr>, min1: bool, cx: Ctx, mode: Mode) -> TokenStream {
        if let (ExprKind::Class(class), None, false, Mode::Recognize) =
            (&e.kind, sep, cx.skipping, mode)
            && let Some(test) = self.byte_test(class)
        {
            return self.class_run(class, test, min1);
        }
        let rt = &self.rt;
        let code = self.expr(e, cx, mode, None);
        let (init, push, result) = match mode {
            Mode::Value => (
                q!(let mut __vs = #rt::Vec::new();),
                q!(__vs.push(__v);),
                q!(Ok((__vs, __p))),
            ),
            Mode::Recognize => (q!(), q!(), q!(Ok(__p))),
        };
        let matched = match mode {
            Mode::Value => q!(Ok((__v, __q))),
            Mode::Recognize => q!(Ok(__q)),
        };
        let sep = match sep {
            Some(sep) => {
                let sep = self.expr(sep, cx, Mode::Recognize, None);
                q! {
                    let __pos = if __first {
                        __pos
                    } else {
                        match #sep {
                            Ok(__q) => __q,
                            Err(_) => break,
                        }
                    };
                }
            }
            None => q!(),
        };
        let finish = if min1 {
            q!(if __first { Err(#rt::Fail) } else { #result })
        } else {
            result
        };
        q! {{
            #init
            let mut __p = __pos;
            let mut __first = true;
            loop {
                let __pos = __p;
                #sep
                match #code {
                    #matched => {
                        #push
                        let __progress = __q != __p;
                        __first = false;
                        __p = __q;
                        if !__progress {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            #finish
        }}
    }

    fn seq(
        &self,
        elems: &[Expr],
        action: Option<(Span, bool)>,
        cx: Ctx,
        mode: Mode,
        ret: Option<&TokenStream>,
    ) -> TokenStream {
        let rt = &self.rt;
        let label = self.label();
        let names: Vec<&str> = elems
            .iter()
            .filter_map(|e| match &e.kind {
                ExprKind::Bind { name, .. } => Some(name.name.as_str()),
                _ => None,
            })
            .collect();
        let pred = action.is_some_and(|(_, pred)| pred);
        // Plain actions only run when the value is needed; predicates always run.
        let code = match action {
            Some((code, _)) if mode == Mode::Value || pred => Some(self.tokens(code)),
            _ => None,
        };
        let usage = match &code {
            Some(code) => {
                let mut usage = Usage::default();
                scan_action(code.clone(), &names, false, &mut usage);
                usage
            }
            None => Usage::default(),
        };
        let at = self.after_skip(cx);

        let mut stmts = Vec::new();
        // A predicate needs `__start` outside the block.
        if code.is_some() && (usage.whole_text || usage.whole_span) && !pred {
            stmts.push(q!(let __start = #at;));
        }
        let mut tuple = Vec::new();
        for (i, el) in elems.iter().enumerate() {
            let (name, inner) = match &el.kind {
                ExprKind::Bind { name, expr } if code.is_some() => (Some(name), &**expr),
                ExprKind::Bind { expr, .. } => (None, &**expr),
                _ => (None, el),
            };
            let uses = |set: &HashSet<String>| name.is_some_and(|n| set.contains(&n.name));
            let (need_value, need_text, need_span) = if code.is_some() {
                (uses(&usage.value), uses(&usage.text), uses(&usage.span))
            } else {
                (
                    action.is_none() && mode == Mode::Value && !self.an.silent(inner),
                    false,
                    false,
                )
            };
            let el_mode = if need_value {
                Mode::Value
            } else {
                Mode::Recognize
            };
            let el_code = self.expr(inner, cx, el_mode, None);
            if need_text || need_span {
                let start = start_var(&name.unwrap().name);
                stmts.push(q!(let #start = #at;));
            }
            if need_value {
                let var = match name {
                    Some(name) => self.user_ident(name),
                    None => {
                        let v = ident(&format!("__v{i}"));
                        tuple.push(v.clone());
                        v
                    }
                };
                stmts.push(q! {
                    let (#var, __pos) = match #el_code {
                        Ok(__r) => __r,
                        Err(__f) => break #label Err(__f),
                    };
                });
            } else {
                stmts.push(q! {
                    let __pos = match #el_code {
                        Ok(__p) => __p,
                        Err(__f) => break #label Err(__f),
                    };
                });
            }
            if let Some(name) = name {
                let start = start_var(&name.name);
                if need_text {
                    let var = text_var(&name.name);
                    stmts.push(q!(let #var = __cx.slice(#start, __pos);));
                }
                if need_span {
                    let var = span_var(&name.name);
                    stmts.push(q!(let #var = #rt::span(#start, __pos);));
                }
            }
        }

        let result = match code {
            Some(code) => {
                if usage.whole_text {
                    let var = text_var("_");
                    stmts.push(q!(let #var = __cx.slice(__start, __pos);));
                }
                if usage.whole_span {
                    let var = span_var("_");
                    stmts.push(q!(let #var = #rt::span(__start, __pos);));
                }
                let code = rewrite_action(code, &names);
                // The value of a predicate is not the value of the sequence.
                let ret = ret.filter(|_| !pred).map(|ty| q!(-> #ty));
                q!(Ok(((|| #ret #code)(), __pos)))
            }
            None => match (mode, tuple.as_slice()) {
                (Mode::Recognize, _) => q!(Ok(__pos)),
                (Mode::Value, []) => q!(Ok(((), __pos))),
                (Mode::Value, [v]) => q!(Ok((#v, __pos))),
                (Mode::Value, vs) => q!(Ok(((#(#vs),*), __pos))),
            },
        };
        let block = q!(#label: { #(#stmts)* #result });
        if !pred {
            return block;
        }
        // See `Cx::fail_predicate` for the errors of a sequence that fails its predicate.
        let ok = match mode {
            Mode::Value => q!(Ok((__v, __pos))),
            Mode::Recognize => q!(Ok(__pos)),
        };
        q! {{
            let __start = #at;
            let __mark = __cx.mark();
            match #block {
                Ok((__o, __pos)) => match #rt::Predicate::into_outcome(__o) {
                    Ok(__v) => {
                        __cx.commit(__mark);
                        #ok
                    }
                    Err(__message) => Err(__cx.fail_predicate(__mark, __start, __message)),
                },
                Err(__f) => {
                    __cx.commit(__mark);
                    Err(__f)
                }
            }
        }}
    }
}

// Whether `value` is in the ranges of `class`, ignoring whether the class is negated.
fn class_test(class: &Class, value: TokenStream, lit: impl Fn(char) -> Literal) -> TokenStream {
    if class.ranges.is_empty() {
        return q!(false);
    }
    let pats = class.ranges.iter().map(|&(lo, hi)| {
        let (l, h) = (lit(lo), lit(hi));
        if lo == hi { q!(#l) } else { q!(#l..=#h) }
    });
    q!(::core::matches!(#value, #(#pats)|*))
}

fn unit_ok(mode: Mode) -> TokenStream {
    match mode {
        Mode::Value => q!(Ok(((), __pos))),
        Mode::Recognize => q!(Ok(__pos)),
    }
}

// How an action refers to the bindings of its sequence.
#[derive(Default)]
struct Usage {
    value: HashSet<String>,
    text: HashSet<String>,
    span: HashSet<String>,
    whole_text: bool,
    whole_span: bool,
}

// Macros whose `#x` means interpolation, which `#x` of actions must not be confused with.
const QUOTE_MACROS: &[&str] = &[
    "quote",
    "quote_spanned",
    "parse_quote",
    "parse_quote_spanned",
];

// `#x` or `@x` at `tts[i]`: the sigil and the name. A sigil after an operand is not one, as in
// the pattern `n @ x`.
fn sigil_at(tts: &[TokenTree], i: usize) -> Option<(char, &Ident)> {
    let sigil = match &tts[i] {
        TokenTree::Punct(p) if matches!(p.as_char(), '#' | '@') => p.as_char(),
        _ => return None,
    };
    let Some(TokenTree::Ident(id)) = tts.get(i + 1) else {
        return None;
    };
    let after_operand = i
        .checked_sub(1)
        .is_some_and(|j| matches!(tts[j], TokenTree::Ident(_) | TokenTree::Literal(_)));
    (!after_operand).then_some((sigil, id))
}

fn is_quote_invocation(before: &[TokenTree]) -> bool {
    matches!(before, [.., TokenTree::Ident(m), TokenTree::Punct(bang)]
        if bang.as_char() == '!' && QUOTE_MACROS.iter().any(|q| m == q))
}

fn scan_action(tokens: TokenStream, names: &[&str], in_quote: bool, usage: &mut Usage) {
    let tts: Vec<TokenTree> = tokens.into_iter().collect();
    let mut i = 0;
    while i < tts.len() {
        if !in_quote && let Some((sigil, id)) = sigil_at(&tts, i) {
            let name = id.to_string();
            match (sigil, name == "_") {
                ('#', true) => usage.whole_text = true,
                ('@', true) => usage.whole_span = true,
                ('#', false) => {
                    usage.text.insert(name);
                }
                _ => {
                    usage.span.insert(name);
                }
            }
            i += 2;
            continue;
        }
        match &tts[i] {
            TokenTree::Ident(id) => {
                let name = id.to_string();
                if names.contains(&name.as_str()) {
                    usage.value.insert(name);
                }
            }
            TokenTree::Group(g) => {
                let quote = in_quote || is_quote_invocation(&tts[..i]);
                scan_action(g.stream(), names, quote, usage);
            }
            _ => {}
        }
        i += 1;
    }
}

// Replaces `#x`, `@x`, `#_` and `@_` with the variables that hold them.
fn rewrite_action(tokens: TokenStream, names: &[&str]) -> TokenStream {
    let tts: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tts.len() {
        if let Some((sigil, id)) = sigil_at(&tts, i) {
            let name = id.to_string();
            let var = if name == "_" || names.contains(&name.as_str()) {
                let var = if sigil == '#' {
                    text_var(&name)
                } else {
                    span_var(&name)
                };
                let span = TokenSpan::mixed_site().located_at(id.span());
                TokenStream::from(TokenTree::Ident(Ident::new(&var.to_string(), span)))
            } else {
                let message =
                    format!("`{sigil}{name}` does not refer to a binding of this sequence");
                quote_spanned!(id.span()=> ::core::compile_error!(#message))
            };
            out.extend(var);
            i += 2;
            continue;
        }
        match &tts[i] {
            TokenTree::Group(g) => {
                let stream = if is_quote_invocation(&tts[..i]) {
                    g.stream()
                } else {
                    rewrite_action(g.stream(), names)
                };
                let mut group = Group::new(g.delimiter(), stream);
                group.set_span(g.span());
                out.push(TokenTree::Group(group));
            }
            tt => out.push(tt.clone()),
        }
        i += 1;
    }
    out.into_iter().collect()
}
