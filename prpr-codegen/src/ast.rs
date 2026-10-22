//! The syntax tree of grammars. Spans are byte ranges of the grammar text.

/// A range of byte offsets of the grammar text.
pub type Span = core::range::Range<usize>;

#[derive(Clone, Debug)]
pub struct Grammar {
    /// Inner attributes, `#![...]`.
    pub attrs: Vec<Attr>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Rule(Rule),
    /// A Rust item, copied to the output as it is.
    Rust(Span),
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub span: Span,
    pub attrs: Vec<Attr>,
    /// `pub`, `pub(crate)`, ...
    pub vis: Option<Span>,
    pub atom: bool,
    pub name: Ident,
    pub ty: Option<Span>,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Attr {
    pub span: Span,
    pub name: Ident,
    pub arg: AttrArg,
}

#[derive(Clone, Debug)]
pub enum AttrArg {
    /// `#[memo]`
    None,
    /// `#[label = "..."]`, and doc comments as `doc = "..."`.
    Lit(Lit, Span),
    /// `#![input = bytes]`
    Ident(Ident),
    /// `#[boundary(expr)]`
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    /// `e1 / e2 / ...`, with at least two alternatives.
    Choice(Vec<Expr>),
    /// `e1 e2 ...`, with at least two elements.
    Seq(Vec<Expr>),
    /// `e { code }` or `e &{ code }`. `code` is the span of the braces and what they enclose.
    /// `expr` is an empty sequence if the action has no expression before it.
    Action {
        expr: Box<Expr>,
        code: Span,
        pred: bool,
    },
    /// `name:e`
    Bind {
        name: Ident,
        expr: Box<Expr>,
    },
    /// `&e`
    And(Box<Expr>),
    /// `!e`
    Not(Box<Expr>),
    /// `e?`
    Opt(Box<Expr>),
    /// `e*`
    Many(Box<Expr>),
    /// `e+`
    Many1(Box<Expr>),
    /// `(e, sep)*` and `(e, sep)+`
    SepBy {
        expr: Box<Expr>,
        sep: Box<Expr>,
        min1: bool,
    },
    /// `<e>`
    Atomic(Box<Expr>),
    Lit(Lit),
    Class(Class),
    /// `.`
    Any,
    Ref(Path),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lit {
    Str(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug)]
pub struct Class {
    pub negated: bool,
    /// Inclusive ranges. With byte input, characters up to `'\u{ff}'` stand for bytes.
    pub ranges: Vec<(char, char)>,
    /// The class as written, for error messages.
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Path {
    pub leading_colon: bool,
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl Path {
    /// The rule name, if the path is a single identifier.
    pub fn as_local(&self) -> Option<&Ident> {
        match self.segments.as_slice() {
            [name] if !self.leading_colon => Some(name),
            _ => None,
        }
    }
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Calls `f` on each direct subexpression.
    pub fn for_each_child<'a>(&'a self, mut f: impl FnMut(&'a Expr)) {
        match &self.kind {
            ExprKind::Choice(es) | ExprKind::Seq(es) => es.iter().for_each(f),
            ExprKind::Action { expr, .. }
            | ExprKind::Bind { expr, .. }
            | ExprKind::And(expr)
            | ExprKind::Not(expr)
            | ExprKind::Opt(expr)
            | ExprKind::Many(expr)
            | ExprKind::Many1(expr)
            | ExprKind::Atomic(expr) => f(expr),
            ExprKind::SepBy { expr, sep, .. } => {
                f(expr);
                f(sep);
            }
            ExprKind::Lit(_) | ExprKind::Class(_) | ExprKind::Any | ExprKind::Ref(_) => {}
        }
    }
}

impl Rule {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|a| a.name.name == name)
    }
}

/// Rust keywords, which cannot be rule names.
pub const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

pub fn is_keyword(s: &str) -> bool {
    s == "atom" || KEYWORDS.contains(&s)
}
