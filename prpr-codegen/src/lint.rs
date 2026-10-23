//! The checks of `#![lint]`, for mistakes that are easy to make with PEGs.
//!
//! Unused rules are found by the compiler: with `#![lint]`, the generated functions of private
//! rules that no other rule refers to deny `dead_code`.

use crate::analysis::{Analysis, Boundary, InputKind, RuleInfo, first_calls};
use crate::ast::*;
use crate::diag::Diagnostic;
use std::collections::HashSet;

/// Checks the rules of `an` for unreachable alternatives, alternatives that never match, and
/// rules that alternatives may parse more than once at the same position.
pub fn lint(an: &Analysis) -> Vec<Diagnostic> {
    let mut lint = Lint {
        an,
        succeeds: Vec::new(),
        starts: Vec::new(),
        diags: Vec::new(),
    };
    lint.succeeds = always_succeeding_rules(an);
    lint.starts = (0..an.rules.len()).map(|i| lint.reach(i)).collect();
    for info in an.rules.iter().filter(|r| !r.builtin) {
        lint.walk(&info.rule.body, !info.rule.atom);
        if let Boundary::Custom(e) = &info.boundary {
            lint.walk(e, false);
        }
    }
    lint.diags
}

struct Lint<'a> {
    an: &'a Analysis,
    /// Whether each rule always succeeds.
    succeeds: Vec<bool>,
    /// The rules that each rule may invoke at its start position, directly or not, without
    /// going through rules whose results are memoized.
    starts: Vec<HashSet<usize>>,
    diags: Vec<Diagnostic>,
}

impl Lint<'_> {
    // `skipping`: whether whitespace is skipped before tokens, i.e. `e` is in a syntactic rule.
    fn walk(&mut self, e: &Expr, skipping: bool) {
        if let ExprKind::Choice(alts) = &e.kind {
            self.choice(alts, skipping);
        }
        let skipping = skipping && !matches!(e.kind, ExprKind::Atomic(_));
        e.for_each_child(|c| self.walk(c, skipping));
    }

    fn choice(&mut self, alts: &[Expr], skipping: bool) {
        // An alternative after one that always succeeds is never tried.
        if let Some(i) = alts[..alts.len() - 1]
            .iter()
            .position(|a| succeeds(self.an, &self.succeeds, a, skipping))
        {
            self.diags.push(
                Diagnostic::error(
                    alts[i + 1].span,
                    "this alternative is never tried, because the one before it always succeeds",
                )
                .note_at(alts[i].span, "this alternative always succeeds"),
            );
            return;
        }

        let mut reported = HashSet::new();
        for j in 1..alts.len() {
            let later = elements(&alts[j]);
            // An alternative whose start an earlier one matches never matches.
            if let Some(d) = (0..j)
                .find_map(|i| self.shadow(&elements(&alts[i]), &later, alts[j].span, skipping))
            {
                self.diags.push(d);
                continue;
            }
            // Rules that an earlier alternative may already have parsed at the same position.
            for earlier in &alts[..j] {
                for (rule, span) in self.reparsed(&elements(earlier), &later, skipping) {
                    if reported.insert(rule) {
                        let name = &self.an.rules[rule].rule.name.name;
                        self.diags.push(
                            Diagnostic::error(
                                span,
                                format!(
                                    "`{name}` may be parsed more than once at the same position by the alternatives of this choice"
                                ),
                            )
                            .note(format!(
                                "add `#[memo]` to `{name}`, factor out what the alternatives start with, or add `#[no_memo]` to `{name}` if this is intended"
                            )),
                        );
                    }
                }
            }
        }
    }

    // Whether the alternative `earlier` matches the start of whatever `later` matches, so that
    // `later` never matches. Then `later` starts with the elements of `earlier`, except that the
    // last one may be a literal that the corresponding literal of `later` starts with.
    fn shadow(
        &self,
        earlier: &[&Expr],
        later: &[&Expr],
        later_span: Span,
        skipping: bool,
    ) -> Option<Diagnostic> {
        let (last, init) = earlier.split_last()?;
        if later.len() < earlier.len() || !init.iter().zip(later).all(|(a, b)| same(a, b)) {
            return None;
        }
        let other = later[init.len()];
        if same(last, other) {
            return Some(Diagnostic::error(
                later_span,
                "this alternative never matches, because an alternative before it matches its start",
            ));
        }
        let (ExprKind::Lit(short), ExprKind::Lit(long)) = (&strip(last).kind, &strip(other).kind)
        else {
            return None;
        };
        let (short_units, long_units) = (self.units(short), self.units(long));
        if !long_units.starts_with(&short_units)
            || self.delimited(&short_units, &long_units, skipping)
        {
            return None;
        }
        Some(
            Diagnostic::error(
                strip(other).span,
                format!(
                    "{} never matches here, because {} before it matches its start",
                    show(long),
                    show(short)
                ),
            )
            .note("put longer literals first"),
        )
    }

    // Whether, in a syntactic rule, `short` fails where `long` is, because `short` ends with a
    // word character and `long` continues with one. Unknown when `word_char` is not a class.
    fn delimited(&self, short: &[char], long: &[char], skipping: bool) -> bool {
        if !skipping {
            return false;
        }
        let Some(class) = &self.an.word_class else {
            return true;
        };
        match (short.last(), long.get(short.len())) {
            (Some(&a), Some(&b)) => class.contains(a) && class.contains(b),
            _ => false,
        }
    }

    // A literal as the characters (or bytes, as characters up to `'\u{ff}'`) it matches.
    fn units(&self, lit: &Lit) -> Vec<char> {
        match (lit, self.an.input) {
            (Lit::Str(s), InputKind::Str) => s.chars().collect(),
            (Lit::Str(s), InputKind::Bytes) => s.bytes().map(char::from).collect(),
            (Lit::Bytes(b), _) => b.iter().map(|&b| char::from(b)).collect(),
        }
    }

    // The syntactic rules that `later` may parse again at the position where `earlier` parsed
    // them, with where to report them: the rules in what both alternatives start with, and the
    // rules that both invoke at the start of what follows.
    fn reparsed(&self, earlier: &[&Expr], later: &[&Expr], skipping: bool) -> Vec<(usize, Span)> {
        let common = earlier
            .iter()
            .zip(later)
            .take_while(|(a, b)| same(a, b))
            .count();
        let mut out = Vec::new();
        for e in &later[..common] {
            self.direct_refs(e, &mut out);
        }
        if let Some(next) = later.get(common) {
            let a = self.start_rules(&earlier[common..], skipping);
            let b = self.start_rules(&later[common..], skipping);
            let both: HashSet<usize> = a.intersection(&b).copied().collect();
            // The outermost ones: memoizing them covers the rules they invoke.
            for &r in &both {
                if !both.iter().any(|&s| s != r && self.starts[s].contains(&r)) {
                    out.push((r, next.span));
                }
            }
        }
        out.retain(|&(r, _)| self.candidate(&self.an.rules[r]));
        out.sort_by_key(|&(r, _)| r);
        out
    }

    // Whether parsing the rule again is worth reporting.
    fn candidate(&self, info: &RuleInfo) -> bool {
        !info.builtin && !info.rule.atom && !info.memo && !info.no_memo
    }

    fn direct_refs(&self, e: &Expr, out: &mut Vec<(usize, Span)>) {
        if let ExprKind::Ref(p) = &e.kind
            && let Some(id) = p.as_local()
        {
            out.push((self.an.index[&id.name], e.span));
        }
        e.for_each_child(|c| self.direct_refs(c, out));
    }

    // The rules that a sequence of elements may invoke at its start position, directly or not.
    fn start_rules(&self, elems: &[&Expr], skipping: bool) -> HashSet<usize> {
        let mut direct = Vec::new();
        for e in elems {
            first_calls(self.an, e, skipping, &mut direct);
            if !self.an.nullable(e) {
                break;
            }
        }
        let mut out = HashSet::new();
        for r in direct {
            out.insert(r);
            out.extend(&self.starts[r]);
        }
        out
    }

    // The rules that rule `i` may invoke at its start position, directly or not, without going
    // through memoized rules, whose bodies do not run again. There is no left recursion, so this
    // terminates.
    fn reach(&self, i: usize) -> HashSet<usize> {
        let mut out = HashSet::new();
        let mut stack = vec![i];
        while let Some(r) = stack.pop() {
            let info = &self.an.rules[r];
            if r != i && (info.memo || info.no_memo) {
                continue;
            }
            let mut next = Vec::new();
            first_calls(self.an, &info.rule.body, !info.rule.atom, &mut next);
            for n in next {
                if out.insert(n) {
                    stack.push(n);
                }
            }
        }
        out
    }
}

// The elements of an alternative, without its action.
fn elements(e: &Expr) -> Vec<&Expr> {
    match &e.kind {
        ExprKind::Seq(es) => es.iter().collect(),
        ExprKind::Action {
            expr, pred: false, ..
        } => match &expr.kind {
            ExprKind::Seq(es) if es.is_empty() => Vec::new(),
            _ => elements(expr),
        },
        _ => vec![e],
    }
}

// `e` without a binding.
fn strip(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Bind { expr, .. } => strip(expr),
        _ => e,
    }
}

// Whether two expressions match the same input, judged by their structure: bindings and actions
// do not matter, and semantic predicates are never the same.
fn same(a: &Expr, b: &Expr) -> bool {
    use ExprKind::*;
    match (&strip(a).kind, &strip(b).kind) {
        (Action { pred: true, .. }, _) | (_, Action { pred: true, .. }) => false,
        (Action { expr: a, .. }, _) => same(a, b),
        (_, Action { expr: b, .. }) => same(a, b),
        (Choice(a), Choice(b)) | (Seq(a), Seq(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (And(a), And(b))
        | (Not(a), Not(b))
        | (Opt(a), Opt(b))
        | (Many(a), Many(b))
        | (Many1(a), Many1(b))
        | (Atomic(a), Atomic(b)) => same(a, b),
        (
            SepBy {
                expr: a,
                sep: sa,
                min1: ma,
            },
            SepBy {
                expr: b,
                sep: sb,
                min1: mb,
            },
        ) => ma == mb && same(a, b) && same(sa, sb),
        (Lit(a), Lit(b)) => a == b,
        (Class(a), Class(b)) => a.negated == b.negated && a.ranges == b.ranges,
        (Any, Any) => true,
        (Ref(a), Ref(b)) => {
            a.leading_colon == b.leading_colon
                && a.segments.len() == b.segments.len()
                && a.segments
                    .iter()
                    .zip(&b.segments)
                    .all(|(a, b)| a.name == b.name)
        }
        _ => false,
    }
}

// The rules that always succeed, as the least fixpoint, so that recursion counts as failing.
fn always_succeeding_rules(an: &Analysis) -> Vec<bool> {
    let mut known = vec![false; an.rules.len()];
    loop {
        let mut changed = false;
        for (i, info) in an.rules.iter().enumerate() {
            if !known[i] && succeeds(an, &known, &info.rule.body, !info.rule.atom) {
                known[i] = true;
                changed = true;
            }
        }
        if !changed {
            return known;
        }
    }
}

// Whether `e` certainly succeeds. Tokens in syntactic rules may fail their delimiter checks.
fn succeeds(an: &Analysis, rules: &[bool], e: &Expr, skipping: bool) -> bool {
    match &e.kind {
        ExprKind::Lit(Lit::Str(s)) => s.is_empty(),
        ExprKind::Lit(Lit::Bytes(b)) => b.is_empty(),
        ExprKind::Class(_) | ExprKind::Any | ExprKind::Not(_) => false,
        ExprKind::Ref(p) => {
            let Some(info) = an.resolve(p) else {
                return false;
            };
            let i = an.index[&info.rule.name.name];
            rules[i] && !(skipping && info.rule.atom && !matches!(info.boundary, Boundary::None))
        }
        ExprKind::Seq(es) => es.iter().all(|e| succeeds(an, rules, e, skipping)),
        ExprKind::Choice(es) => es.iter().any(|e| succeeds(an, rules, e, skipping)),
        ExprKind::Action { pred: true, .. } => false,
        ExprKind::Action { expr, .. } | ExprKind::Bind { expr, .. } | ExprKind::And(expr) => {
            succeeds(an, rules, expr, skipping)
        }
        ExprKind::Many1(expr)
        | ExprKind::SepBy {
            expr, min1: true, ..
        } => succeeds(an, rules, expr, skipping),
        ExprKind::Opt(_) | ExprKind::Many(_) | ExprKind::SepBy { min1: false, .. } => true,
        ExprKind::Atomic(expr) => !skipping && succeeds(an, rules, expr, false),
    }
}

// A literal as written in a grammar, roughly.
fn show(lit: &Lit) -> String {
    match lit {
        Lit::Str(s) => format!("`{s:?}`"),
        Lit::Bytes(b) => format!("`b\"{}\"`", b.escape_ascii()),
    }
}
