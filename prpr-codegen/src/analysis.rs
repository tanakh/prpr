//! Static checks and the facts about rules that code generation needs.

use crate::ast::*;
use crate::diag::Diagnostic;
use std::collections::HashMap;

/// The rules that implicit whitespace and delimiter checks are made of.
pub const SPACES: &str = "spaces";
pub const WHITESPACE: &str = "whitespace";
pub const COMMENT: &str = "comment";
pub const WORD_CHAR: &str = "word_char";
const SPECIAL_RULES: &[&str] = &[SPACES, WHITESPACE, COMMENT, WORD_CHAR];
/// The rules whose failures are not reported where they start: whitespace is never expected,
/// but the end of a comment that started is.
pub const QUIET_RULES: &[&str] = &[SPACES, WHITESPACE, COMMENT];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    Str,
    Bytes,
}

#[derive(Clone, Debug)]
pub enum Boundary {
    /// Rejects a token that ends with a word character followed by a word character.
    Default,
    None,
    /// `#[boundary(e)]`: `e` must match after the token.
    Custom(Expr),
}

#[derive(Clone, Debug)]
pub struct RuleInfo {
    pub rule: Rule,
    /// Defined by prpr, not by the grammar.
    pub builtin: bool,
    /// Whether the rule has a value, i.e. a type other than `()`.
    pub typed: bool,
    pub nullable: bool,
    pub memo: bool,
    /// `#[no_memo]`: not memoized on purpose, which silences the memoization lint.
    pub no_memo: bool,
    /// Whether another rule of the grammar refers to the rule.
    pub referenced: bool,
    pub label: Option<String>,
    pub boundary: Boundary,
    pub docs: Vec<String>,
}

#[derive(Debug)]
pub struct Analysis {
    pub rules: Vec<RuleInfo>,
    pub index: HashMap<String, usize>,
    pub input: InputKind,
    pub rust_items: Vec<Span>,
    /// `word_char` when it is a plain character class, which allows deciding at generation time
    /// whether a literal ends with a word character.
    pub word_class: Option<Class>,
    /// `#![lint]`: check for mistakes that are easy to make with PEGs.
    pub lint: bool,
    /// `#![profile]`: count the calls of rules.
    pub profile: bool,
}

/// The attributes of a grammar.
struct GrammarAttrs {
    input: InputKind,
    lint: bool,
    profile: bool,
}

impl Analysis {
    pub fn rule(&self, name: &str) -> Option<&RuleInfo> {
        self.index.get(name).map(|&i| &self.rules[i])
    }

    /// The rule a reference refers to.
    pub fn resolve(&self, path: &Path) -> Option<&RuleInfo> {
        path.as_local().and_then(|id| self.rule(&id.name))
    }

    pub fn nullable(&self, e: &Expr) -> bool {
        nullable(e, &|p| self.resolve(p).is_some_and(|r| r.nullable))
    }

    /// Whether the value of `e` is left out of the tuple of a sequence without an action.
    pub fn silent(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Lit(_) | ExprKind::And(_) | ExprKind::Not(_) => true,
            ExprKind::Class(_) | ExprKind::Any | ExprKind::Action { .. } => false,
            ExprKind::Ref(p) => self.resolve(p).is_some_and(|r| !r.typed),
            ExprKind::Seq(es) => es.iter().all(|e| self.silent(e)),
            ExprKind::Choice(es) => !is_literal_choice(es) && es.iter().all(|e| self.silent(e)),
            ExprKind::Bind { expr, .. }
            | ExprKind::Opt(expr)
            | ExprKind::Many(expr)
            | ExprKind::Many1(expr)
            | ExprKind::Atomic(expr)
            | ExprKind::SepBy { expr, .. } => self.silent(expr),
        }
    }
}

/// A choice between literals, which yields the matched literal.
pub fn is_literal_choice(alts: &[Expr]) -> bool {
    alts.iter().all(|e| matches!(e.kind, ExprKind::Lit(_)))
}

fn nullable(e: &Expr, rule_nullable: &dyn Fn(&Path) -> bool) -> bool {
    match &e.kind {
        ExprKind::Lit(Lit::Str(s)) => s.is_empty(),
        ExprKind::Lit(Lit::Bytes(b)) => b.is_empty(),
        ExprKind::Class(_) | ExprKind::Any => false,
        ExprKind::Ref(p) => rule_nullable(p),
        ExprKind::Seq(es) => es.iter().all(|e| nullable(e, rule_nullable)),
        ExprKind::Choice(es) => es.iter().any(|e| nullable(e, rule_nullable)),
        ExprKind::And(_) | ExprKind::Not(_) | ExprKind::Opt(_) | ExprKind::Many(_) => true,
        ExprKind::Action { expr, .. }
        | ExprKind::Bind { expr, .. }
        | ExprKind::Many1(expr)
        | ExprKind::Atomic(expr) => nullable(expr, rule_nullable),
        ExprKind::SepBy { expr, min1, .. } => !min1 || nullable(expr, rule_nullable),
    }
}

/// Analyzes a grammar parsed from `text`.
pub fn analyze(g: &Grammar, text: &str) -> Result<Analysis, Vec<Diagnostic>> {
    let mut diags = Vec::new();
    let attrs = grammar_attrs(g, &mut diags);
    let input = attrs.input;

    let mut rules = Vec::new();
    let mut rust_items = Vec::new();
    let mut index = HashMap::new();
    for item in &g.items {
        match item {
            Item::Rust(span) => rust_items.push(*span),
            Item::Rule(rule) => {
                if is_keyword(&rule.name.name) {
                    diags.push(Diagnostic::error(
                        rule.name.span,
                        format!(
                            "`{}` is a keyword and cannot be a rule name",
                            rule.name.name
                        ),
                    ));
                }
                if let Some(&first) = index.get(&rule.name.name) {
                    let first: &RuleInfo = &rules[first];
                    diags.push(
                        Diagnostic::error(
                            rule.name.span,
                            format!("rule `{}` is defined more than once", rule.name.name),
                        )
                        .note_at(first.rule.name.span, "the first definition is here"),
                    );
                    continue;
                }
                index.insert(rule.name.name.clone(), rules.len());
                rules.push(rule_info(rule.clone(), false, text, &mut diags));
            }
        }
    }

    for rule in prelude(input, &index) {
        index.insert(rule.name.name.clone(), rules.len());
        rules.push(rule_info(rule, true, text, &mut diags));
    }

    for info in &rules {
        let rule = &info.rule;
        if SPECIAL_RULES.contains(&rule.name.name.as_str()) && !rule.atom {
            diags.push(
                Diagnostic::error(
                    rule.name.span,
                    format!("`{}` must be an `atom` rule", rule.name.name),
                )
                .note("it is used for implicit whitespace and delimiters"),
            );
        }
        let boundary_attr = rule.attr("boundary").or(rule.attr("no_boundary"));
        if let Some(attr) = boundary_attr
            && !rule.atom
        {
            diags.push(Diagnostic::error(
                attr.span,
                "`#[boundary]` and `#[no_boundary]` only apply to `atom` rules",
            ));
        }
    }

    let mut an = Analysis {
        rules,
        index,
        input,
        rust_items,
        word_class: None,
        lint: attrs.lint,
        profile: attrs.profile,
    };
    an.word_class = match &an.rule(WORD_CHAR).unwrap().rule.body.kind {
        ExprKind::Class(c) => Some(c.clone()),
        _ => None,
    };

    for info in &an.rules {
        let mut check = |e: &Expr| check_expr(&an, e, &mut diags);
        check(&info.rule.body);
        if let Boundary::Custom(e) = &info.boundary {
            check(e);
        }
    }
    if diags.iter().any(Diagnostic::is_error) {
        return Err(diags);
    }

    compute_nullable(&mut an);
    compute_references(&mut an);
    for info in &an.rules {
        check_repetitions(&an, &info.rule.body, &mut diags);
        if let Boundary::Custom(e) = &info.boundary {
            check_repetitions(&an, e, &mut diags);
        }
    }
    check_left_recursion(&an, &mut diags);
    // The lints rely on the absence of left recursion.
    if an.lint && !diags.iter().any(Diagnostic::is_error) {
        diags.extend(crate::lint::lint(&an));
    }

    if diags.iter().any(Diagnostic::is_error) {
        Err(diags)
    } else {
        Ok(an)
    }
}

fn grammar_attrs(g: &Grammar, diags: &mut Vec<Diagnostic>) -> GrammarAttrs {
    let mut attrs = GrammarAttrs {
        input: InputKind::Str,
        lint: false,
        profile: false,
    };
    let mut seen = Vec::new();
    for attr in &g.attrs {
        let name = attr.name.name.as_str();
        if matches!(name, "input" | "lint" | "profile") {
            if seen.contains(&name) {
                diags.push(Diagnostic::error(
                    attr.span,
                    format!("a grammar cannot have more than one `#![{name}]`"),
                ));
                continue;
            }
            seen.push(name);
        }
        match (name, &attr.arg) {
            ("doc", _) => {}
            ("input", AttrArg::Ident(id)) if id.name == "str" => attrs.input = InputKind::Str,
            ("input", AttrArg::Ident(id)) if id.name == "bytes" => attrs.input = InputKind::Bytes,
            ("input", _) => diags.push(Diagnostic::error(
                attr.span,
                "expected `#![input = str]` or `#![input = bytes]`",
            )),
            ("lint", AttrArg::None) => attrs.lint = true,
            ("profile", AttrArg::None) => attrs.profile = true,
            (name @ ("lint" | "profile"), _) => diags.push(Diagnostic::error(
                attr.span,
                format!("expected `#![{name}]` without arguments"),
            )),
            (name, _) => diags.push(Diagnostic::error(
                attr.name.span,
                format!("unknown grammar attribute `{name}`"),
            )),
        }
    }
    attrs
}

fn rule_info(rule: Rule, builtin: bool, text: &str, diags: &mut Vec<Diagnostic>) -> RuleInfo {
    let mut memo = false;
    let mut no_memo = false;
    let mut label = None;
    let mut boundary = Boundary::Default;
    let mut docs = Vec::new();
    let mut seen = Vec::new();
    for attr in &rule.attrs {
        let name = attr.name.name.as_str();
        if matches!(
            name,
            "memo" | "no_memo" | "label" | "boundary" | "no_boundary"
        ) {
            if seen.contains(&name) {
                diags.push(Diagnostic::error(
                    attr.span,
                    format!("a rule cannot have more than one `#[{name}]`"),
                ));
                continue;
            }
            seen.push(name);
        }
        match (name, &attr.arg) {
            ("doc", AttrArg::Lit(Lit::Str(s))) => docs.push(s.clone()),
            ("memo", AttrArg::None) => memo = true,
            ("no_memo", AttrArg::None) => no_memo = true,
            ("no_boundary", AttrArg::None) => boundary = Boundary::None,
            ("label", AttrArg::Lit(Lit::Str(s))) => label = Some(s.clone()),
            ("boundary", AttrArg::Expr(e)) => boundary = Boundary::Custom(e.clone()),
            (name @ ("doc" | "label"), _) => diags.push(Diagnostic::error(
                attr.span,
                format!("expected `#[{name} = \"...\"]`"),
            )),
            (name @ ("memo" | "no_memo" | "no_boundary"), _) => diags.push(Diagnostic::error(
                attr.span,
                format!("expected `#[{name}]` without arguments"),
            )),
            ("boundary", _) => diags.push(Diagnostic::error(
                attr.span,
                "expected `#[boundary(expression)]`",
            )),
            (name, _) => diags.push(Diagnostic::error(
                attr.name.span,
                format!("unknown attribute `{name}`"),
            )),
        }
    }
    for (a, b) in [("memo", "no_memo"), ("boundary", "no_boundary")] {
        if seen.contains(&a) && seen.contains(&b) {
            diags.push(Diagnostic::error(
                rule.name.span,
                format!("a rule cannot have both `#[{a}]` and `#[{b}]`"),
            ));
        }
    }
    // A rule of type `()` is the same as a rule without a type.
    let typed = rule.ty.is_some_and(|ty| {
        let ty: String = text[ty].chars().filter(|c| !c.is_whitespace()).collect();
        ty != "()"
    });
    RuleInfo {
        rule,
        builtin,
        typed,
        nullable: false,
        memo,
        no_memo,
        referenced: false,
        label,
        boundary,
        docs,
    }
}

/// The rules that a grammar gets unless it defines them.
fn prelude(input: InputKind, defined: &HashMap<String, usize>) -> Vec<Rule> {
    let sp = Span::default();
    let e = |kind| Expr::new(kind, sp);
    let ident = |name: &str| Ident {
        name: name.to_string(),
        span: sp,
    };
    let reference = |name: &str| {
        e(ExprKind::Ref(Path {
            leading_colon: false,
            segments: vec![ident(name)],
            span: sp,
        }))
    };
    let class = |ranges: &[(char, char)], text: &str| {
        e(ExprKind::Class(Class {
            negated: false,
            ranges: ranges.to_vec(),
            text: text.to_string(),
        }))
    };
    // The empty class, which never matches.
    let never = || class(&[], "[]");

    let whitespace = match input {
        InputKind::Str => class(
            &[(' ', ' '), ('\t', '\t'), ('\r', '\r'), ('\n', '\n')],
            "[' ' '\\t' '\\r' '\\n']",
        ),
        InputKind::Bytes => never(),
    };
    let spaces = match (
        defined.contains_key(WHITESPACE),
        defined.contains_key(COMMENT),
    ) {
        // The common case, written so that it compiles to a tight loop.
        (false, false) => e(ExprKind::Many(Box::new(whitespace.clone()))),
        (_, false) => e(ExprKind::Many(Box::new(reference(WHITESPACE)))),
        (_, true) => e(ExprKind::Many(Box::new(e(ExprKind::Choice(vec![
            reference(WHITESPACE),
            reference(COMMENT),
        ]))))),
    };
    // Binary data has no words, so tokens need no delimiters by default.
    let word_char = match input {
        InputKind::Str => class(
            &[('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
            "[a-zA-Z0-9_]",
        ),
        InputKind::Bytes => never(),
    };
    let eof = e(ExprKind::Not(Box::new(e(ExprKind::Any))));

    [
        (WHITESPACE, whitespace),
        (COMMENT, never()),
        (SPACES, spaces),
        (WORD_CHAR, word_char),
        ("eof", eof),
    ]
    .into_iter()
    .filter(|(name, _)| !defined.contains_key(*name))
    .map(|(name, body)| Rule {
        attrs: vec![],
        vis: None,
        atom: true,
        name: ident(name),
        ty: None,
        body,
    })
    .collect()
}

fn check_expr(an: &Analysis, e: &Expr, diags: &mut Vec<Diagnostic>) {
    match &e.kind {
        ExprKind::Ref(path) => match path.as_local() {
            Some(id) if an.rule(&id.name).is_none() => {
                let mut d = Diagnostic::error(id.span, format!("cannot find rule `{}`", id.name));
                if let Some(similar) = similar_name(an, &id.name) {
                    d = d.note(format!("a rule with a similar name exists: `{similar}`"));
                }
                diags.push(d);
            }
            Some(_) => {}
            None if !path.leading_colon && path.segments[0].name == "self" => {
                diags.push(Diagnostic::error(
                    path.span,
                    "a rule of this grammar is referred to by its name, without `self::`",
                ))
            }
            // A rule of another grammar, which the compiler checks.
            None if path.segments.len() >= 2 => {}
            None => diags.push(Diagnostic::error(
                path.span,
                "a reference to a rule of another grammar needs the path of its module",
            )),
        },
        ExprKind::Lit(Lit::Bytes(_)) if an.input == InputKind::Str => diags.push(
            Diagnostic::error(e.span, "byte literals require `#![input = bytes]`"),
        ),
        ExprKind::Class(c) => {
            if let Some(&(lo, hi)) = c.ranges.iter().find(|&&(lo, hi)| lo > hi) {
                let (lo, hi) = (lo.escape_debug(), hi.escape_debug());
                diags.push(
                    Diagnostic::error(e.span, format!("the range `{lo}-{hi}` is reversed")).note(
                        format!(
                            "a range goes from the lower character to the higher, as in `{hi}-{lo}`"
                        ),
                    ),
                );
            }
            if an.input == InputKind::Bytes && c.ranges.iter().any(|&(_, hi)| hi > '\u{ff}') {
                diags.push(Diagnostic::error(
                    e.span,
                    "a character class for byte input can only contain characters up to `\\u{ff}`",
                ));
            }
        }
        ExprKind::Seq(es) => check_bindings(es, diags),
        ExprKind::Action { expr, .. } if !matches!(expr.kind, ExprKind::Seq(_)) => {
            check_bindings(std::slice::from_ref(expr), diags);
        }
        _ => {}
    }
    e.for_each_child(|c| check_expr(an, c, diags));
}

fn check_bindings(elems: &[Expr], diags: &mut Vec<Diagnostic>) {
    let mut seen: Vec<&str> = Vec::new();
    for el in elems {
        if let ExprKind::Bind { name, .. } = &el.kind {
            if seen.contains(&name.name.as_str()) {
                diags.push(Diagnostic::error(
                    name.span,
                    format!("`{}` is bound more than once in this sequence", name.name),
                ));
            }
            seen.push(&name.name);
        }
    }
}

fn similar_name<'a>(an: &'a Analysis, name: &str) -> Option<&'a str> {
    an.rules
        .iter()
        .map(|r| r.rule.name.name.as_str())
        .map(|n| (edit_distance(n, name), n))
        .filter(|&(d, n)| d <= (name.len().max(n.len()) / 3).max(1))
        .min()
        .map(|(_, n)| n)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

fn compute_nullable(an: &mut Analysis) {
    loop {
        let mut changed = false;
        for i in 0..an.rules.len() {
            if !an.rules[i].nullable && an.nullable(&an.rules[i].rule.body) {
                an.rules[i].nullable = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn check_repetitions(an: &Analysis, e: &Expr, diags: &mut Vec<Diagnostic>) {
    let loops_forever = match &e.kind {
        ExprKind::Many(inner) | ExprKind::Many1(inner) => an.nullable(inner),
        ExprKind::SepBy { expr, sep, .. } => an.nullable(expr) && an.nullable(sep),
        _ => false,
    };
    if loops_forever {
        diags.push(
            Diagnostic::error(
                e.span,
                "repetition of an expression that can match empty input",
            )
            .note("the repetition would never end"),
        );
    }
    e.for_each_child(|c| check_repetitions(an, c, diags));
}

fn compute_references(an: &mut Analysis) {
    fn walk(an: &Analysis, e: &Expr, from: usize, out: &mut Vec<usize>) {
        if let ExprKind::Ref(p) = &e.kind
            && let Some(id) = p.as_local()
            && let Some(&i) = an.index.get(&id.name)
            && i != from
        {
            out.push(i);
        }
        e.for_each_child(|c| walk(an, c, from, out));
    }
    let mut referenced = Vec::new();
    for (i, info) in an.rules.iter().enumerate() {
        walk(an, &info.rule.body, i, &mut referenced);
        if let Boundary::Custom(e) = &info.boundary {
            walk(an, e, i, &mut referenced);
        }
    }
    // Implicit whitespace and delimiter checks use these.
    referenced.extend([an.index[SPACES], an.index[WORD_CHAR]]);
    for i in referenced {
        an.rules[i].referenced = true;
    }
}

/// The rules that may be invoked at the start position of `e`, including the implicit
/// whitespace before tokens.
pub(crate) fn first_calls(an: &Analysis, e: &Expr, skipping: bool, out: &mut Vec<usize>) {
    let spaces = an.index[SPACES];
    match &e.kind {
        ExprKind::Lit(_) | ExprKind::Class(_) | ExprKind::Any => {
            if skipping {
                out.push(spaces);
            }
        }
        ExprKind::Ref(p) => {
            if let Some(id) = p.as_local() {
                let i = an.index[&id.name];
                if skipping && an.rules[i].rule.atom {
                    out.push(spaces);
                }
                out.push(i);
            }
        }
        ExprKind::Atomic(inner) => {
            if skipping {
                out.push(spaces);
            }
            first_calls(an, inner, false, out);
        }
        ExprKind::Seq(es) => {
            for e in es {
                first_calls(an, e, skipping, out);
                if !an.nullable(e) {
                    break;
                }
            }
        }
        ExprKind::Choice(es) => {
            for e in es {
                first_calls(an, e, skipping, out);
            }
        }
        ExprKind::Action { expr, .. }
        | ExprKind::Bind { expr, .. }
        | ExprKind::And(expr)
        | ExprKind::Not(expr)
        | ExprKind::Opt(expr)
        | ExprKind::Many(expr)
        | ExprKind::Many1(expr) => first_calls(an, expr, skipping, out),
        ExprKind::SepBy { expr, sep, .. } => {
            first_calls(an, expr, skipping, out);
            if an.nullable(expr) {
                first_calls(an, sep, skipping, out);
            }
        }
    }
}

fn check_left_recursion(an: &Analysis, diags: &mut Vec<Diagnostic>) {
    let graph: Vec<Vec<usize>> = an
        .rules
        .iter()
        .map(|info| {
            let mut out = Vec::new();
            first_calls(an, &info.rule.body, !info.rule.atom, &mut out);
            if info.label.is_some() && !info.rule.atom {
                out.push(an.index[SPACES]);
            }
            out.sort_unstable();
            out.dedup();
            out
        })
        .collect();

    // Report one cycle per strongly connected component.
    for scc in strongly_connected_components(&graph) {
        let cyclic = scc.len() > 1 || graph[scc[0]].contains(&scc[0]);
        if !cyclic {
            continue;
        }
        let cycle = find_cycle(&graph, &scc);
        let names: Vec<String> = cycle
            .iter()
            .chain(cycle.first())
            .map(|&i| format!("`{}`", an.rules[i].rule.name.name))
            .collect();
        let at = cycle
            .iter()
            .map(|&i| &an.rules[i])
            .find(|r| !r.builtin)
            .unwrap_or(&an.rules[cycle[0]]);
        diags.push(
            Diagnostic::error(
                at.rule.name.span,
                format!("left recursion: {}", names.join(" -> ")),
            )
            .note("a rule must consume input before it is invoked again; left recursion is not supported"),
        );
    }
}

fn strongly_connected_components(graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct Tarjan<'a> {
        graph: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    impl Tarjan<'_> {
        fn visit(&mut self, v: usize) {
            self.index[v] = Some(self.next);
            self.low[v] = self.next;
            self.next += 1;
            self.stack.push(v);
            self.on_stack[v] = true;
            for &w in &self.graph[v] {
                match self.index[w] {
                    None => {
                        self.visit(w);
                        self.low[v] = self.low[v].min(self.low[w]);
                    }
                    Some(iw) if self.on_stack[w] => self.low[v] = self.low[v].min(iw),
                    Some(_) => {}
                }
            }
            if Some(self.low[v]) == self.index[v] {
                let mut scc = Vec::new();
                loop {
                    let w = self.stack.pop().unwrap();
                    self.on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                scc.sort_unstable();
                self.out.push(scc);
            }
        }
    }
    let n = graph.len();
    let mut t = Tarjan {
        graph,
        index: vec![None; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next: 0,
        out: Vec::new(),
    };
    for v in 0..n {
        if t.index[v].is_none() {
            t.visit(v);
        }
    }
    t.out.sort();
    t.out
}

// A cycle through the first node of a strongly connected component.
fn find_cycle(graph: &[Vec<usize>], scc: &[usize]) -> Vec<usize> {
    let start = scc[0];
    let mut prev = HashMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(v) = queue.pop_front() {
        for &w in &graph[v] {
            if w == start {
                let mut path = vec![v];
                let mut cur = v;
                while cur != start {
                    cur = prev[&cur];
                    path.push(cur);
                }
                path.reverse();
                return path;
            }
            if scc.binary_search(&w).is_ok() && !prev.contains_key(&w) {
                prev.insert(w, v);
                queue.push_back(w);
            }
        }
    }
    vec![start]
}
