//! The syntax tree of GraphQL documents that the prpr parser builds, and a summary of documents
//! that all the GraphQL parsers must agree on.
//!
//! Names borrow from the input; strings are decoded. Definitions, fields, selections and
//! directives have spans: the byte ranges of the input where they are.

use std::range::Range;

pub type Span = Range<usize>;

#[derive(Clone, Debug, PartialEq)]
pub struct Document<'a> {
    pub definitions: Vec<Definition<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Definition<'a> {
    Operation(OperationDefinition<'a>),
    Fragment(FragmentDefinition<'a>),
    Schema(SchemaDefinition<'a>),
    SchemaExtension(SchemaDefinition<'a>),
    Type(TypeDefinition<'a>),
    TypeExtension(TypeDefinition<'a>),
    Directive(DirectiveDefinition<'a>),
}

impl<'a> Definition<'a> {
    /// The type system definition with the description before it, and the span of both, or of
    /// `extend` and the extension.
    pub fn described(mut self, description: Option<String>, span: Span) -> Self {
        match &mut self {
            Definition::Schema(d) | Definition::SchemaExtension(d) => {
                d.description = description;
                d.span = span;
            }
            Definition::Type(d) | Definition::TypeExtension(d) => {
                d.description = description;
                d.span = span;
            }
            Definition::Directive(d) => {
                d.description = description;
                d.span = span;
            }
            Definition::Operation(_) | Definition::Fragment(_) => {}
        }
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Query,
    Mutation,
    Subscription,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationDefinition<'a> {
    pub span: Span,
    pub kind: OperationKind,
    pub name: Option<&'a str>,
    pub variables: Vec<VariableDefinition<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableDefinition<'a> {
    pub span: Span,
    pub name: &'a str,
    pub ty: Type<'a>,
    pub default: Option<Value<'a>>,
    pub directives: Vec<Directive<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FragmentDefinition<'a> {
    pub span: Span,
    pub name: &'a str,
    pub type_condition: &'a str,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Selection<'a> {
    Field(Field<'a>),
    FragmentSpread(FragmentSpread<'a>),
    InlineFragment(InlineFragment<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field<'a> {
    pub span: Span,
    pub alias: Option<&'a str>,
    pub name: &'a str,
    pub arguments: Vec<(&'a str, Value<'a>)>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FragmentSpread<'a> {
    pub span: Span,
    pub name: &'a str,
    pub directives: Vec<Directive<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlineFragment<'a> {
    pub span: Span,
    pub type_condition: Option<&'a str>,
    pub directives: Vec<Directive<'a>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Directive<'a> {
    pub span: Span,
    pub name: &'a str,
    pub arguments: Vec<(&'a str, Value<'a>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Variable(&'a str),
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Enum(&'a str),
    List(Vec<Value<'a>>),
    Object(Vec<(&'a str, Value<'a>)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type<'a> {
    Named(&'a str),
    List(Box<Type<'a>>),
    NonNull(Box<Type<'a>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SchemaDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub directives: Vec<Directive<'a>>,
    pub operation_types: Vec<(OperationKind, &'a str)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub name: &'a str,
    pub directives: Vec<Directive<'a>>,
    pub kind: TypeKind<'a>,
}

impl<'a> TypeDefinition<'a> {
    pub fn new(
        span: Span,
        name: &'a str,
        directives: Vec<Directive<'a>>,
        kind: TypeKind<'a>,
    ) -> Self {
        TypeDefinition {
            span,
            description: None,
            name,
            directives,
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind<'a> {
    Scalar,
    Object {
        implements: Vec<&'a str>,
        fields: Vec<FieldDefinition<'a>>,
    },
    Interface {
        implements: Vec<&'a str>,
        fields: Vec<FieldDefinition<'a>>,
    },
    Union {
        members: Vec<&'a str>,
    },
    Enum {
        values: Vec<EnumValueDefinition<'a>>,
    },
    InputObject {
        fields: Vec<InputValueDefinition<'a>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub name: &'a str,
    pub arguments: Vec<InputValueDefinition<'a>>,
    pub ty: Type<'a>,
    pub directives: Vec<Directive<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputValueDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub name: &'a str,
    pub ty: Type<'a>,
    pub default: Option<Value<'a>>,
    pub directives: Vec<Directive<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumValueDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub name: &'a str,
    pub directives: Vec<Directive<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectiveDefinition<'a> {
    pub span: Span,
    pub description: Option<String>,
    pub name: &'a str,
    pub arguments: Vec<InputValueDefinition<'a>>,
    pub repeatable: bool,
    pub locations: Vec<&'a str>,
}

/// The value of a block string from the text between its `"""`s: without the indentation common
/// to its lines but the first, and without blank lines at the start and the end.
pub fn block_string_value(raw: &str) -> String {
    let raw = raw.replace("\\\"\"\"", "\"\"\"");
    let raw = raw.replace("\r\n", "\n");
    let is_blank = |line: &str| line.bytes().all(|b| b == b' ' || b == b'\t');
    let indent = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
    let lines: Vec<&str> = raw.split(['\n', '\r']).collect();
    let common = lines[1..]
        .iter()
        .filter(|line| !is_blank(line))
        .map(|line| indent(line))
        .min()
        .unwrap_or(0);
    let mut lines: Vec<&str> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line
            } else {
                &line[common.min(line.len())..]
            }
        })
        .collect();
    while lines.first().is_some_and(|line| is_blank(line)) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| is_blank(line)) {
        lines.pop();
    }
    lines.join("\n")
}

/// What the parsers must agree on about a definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// `"query"`, `"fragment"`, `"type"`, `"extend type"`, ...
    pub kind: String,
    pub name: Option<String>,
    /// The number of fields, values, union members, arguments of a directive, root operation
    /// types or selections, depending on the kind.
    pub members: usize,
    pub description: Option<String>,
}

/// The summaries of the definitions of a document parsed with prpr.
pub fn summary(doc: &Document<'_>) -> Vec<Summary> {
    doc.definitions
        .iter()
        .map(|d| {
            let (kind, name, members, description) = match d {
                Definition::Operation(o) => {
                    let kind = match o.kind {
                        OperationKind::Query => "query",
                        OperationKind::Mutation => "mutation",
                        OperationKind::Subscription => "subscription",
                    };
                    (kind.to_string(), o.name, o.selection_set.len(), None)
                }
                Definition::Fragment(f) => (
                    "fragment".to_string(),
                    Some(f.name),
                    f.selection_set.len(),
                    None,
                ),
                Definition::Schema(s) => (
                    "schema".to_string(),
                    None,
                    s.operation_types.len(),
                    s.description.clone(),
                ),
                Definition::SchemaExtension(s) => (
                    "extend schema".to_string(),
                    None,
                    s.operation_types.len(),
                    None,
                ),
                Definition::Type(t) => {
                    let (kind, members) = type_kind(&t.kind);
                    (
                        kind.to_string(),
                        Some(t.name),
                        members,
                        t.description.clone(),
                    )
                }
                Definition::TypeExtension(t) => {
                    let (kind, members) = type_kind(&t.kind);
                    (format!("extend {kind}"), Some(t.name), members, None)
                }
                Definition::Directive(d) => (
                    "directive".to_string(),
                    Some(d.name),
                    d.arguments.len(),
                    d.description.clone(),
                ),
            };
            Summary {
                kind,
                name: name.map(str::to_string),
                members,
                description,
            }
        })
        .collect()
}

fn type_kind(kind: &TypeKind<'_>) -> (&'static str, usize) {
    match kind {
        TypeKind::Scalar => ("scalar", 0),
        TypeKind::Object { fields, .. } => ("type", fields.len()),
        TypeKind::Interface { fields, .. } => ("interface", fields.len()),
        TypeKind::Union { members } => ("union", members.len()),
        TypeKind::Enum { values } => ("enum", values.len()),
        TypeKind::InputObject { fields } => ("input", fields.len()),
    }
}
