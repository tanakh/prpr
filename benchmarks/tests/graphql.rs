//! The GraphQL parsers agree on the definitions of documents, as far as they support them.

use prpr_benchmarks::graphql::{self, Summary};
use prpr_benchmarks::prpr_graphql;

type Parse = fn(&str) -> Result<Vec<Summary>, String>;

fn entry(kind: &str, name: Option<&str>, members: usize, description: Option<String>) -> Summary {
    Summary {
        kind: kind.to_string(),
        name: name.map(str::to_string),
        members,
        description,
    }
}

// Type system documents.

fn prpr_schema(src: &str) -> Result<Vec<Summary>, String> {
    let doc = prpr_graphql::type_system_document(src).map_err(|e| e.to_string())?;
    Ok(graphql::summary(&doc))
}

fn graphql_parser_schema(src: &str) -> Result<Vec<Summary>, String> {
    use graphql_parser::schema::{Definition as D, TypeDefinition as T, TypeExtension as E};
    let doc = graphql_parser::parse_schema::<&str>(src).map_err(|e| e.to_string())?;
    let roots = |roots: [Option<&str>; 3]| roots.iter().flatten().count();
    Ok(doc
        .definitions
        .iter()
        .map(|d| match d {
            D::SchemaDefinition(s) => entry(
                "schema",
                None,
                roots([s.query, s.mutation, s.subscription]),
                None,
            ),
            D::TypeDefinition(T::Scalar(t)) => {
                entry("scalar", Some(t.name), 0, t.description.clone())
            }
            D::TypeDefinition(T::Object(t)) => {
                entry("type", Some(t.name), t.fields.len(), t.description.clone())
            }
            D::TypeDefinition(T::Interface(t)) => entry(
                "interface",
                Some(t.name),
                t.fields.len(),
                t.description.clone(),
            ),
            D::TypeDefinition(T::Union(t)) => {
                entry("union", Some(t.name), t.types.len(), t.description.clone())
            }
            D::TypeDefinition(T::Enum(t)) => {
                entry("enum", Some(t.name), t.values.len(), t.description.clone())
            }
            D::TypeDefinition(T::InputObject(t)) => {
                entry("input", Some(t.name), t.fields.len(), t.description.clone())
            }
            D::TypeExtension(E::Scalar(t)) => entry("extend scalar", Some(t.name), 0, None),
            D::TypeExtension(E::Object(t)) => {
                entry("extend type", Some(t.name), t.fields.len(), None)
            }
            D::TypeExtension(E::Interface(t)) => {
                entry("extend interface", Some(t.name), t.fields.len(), None)
            }
            D::TypeExtension(E::Union(t)) => {
                entry("extend union", Some(t.name), t.types.len(), None)
            }
            D::TypeExtension(E::Enum(t)) => {
                entry("extend enum", Some(t.name), t.values.len(), None)
            }
            D::TypeExtension(E::InputObject(t)) => {
                entry("extend input", Some(t.name), t.fields.len(), None)
            }
            D::DirectiveDefinition(d) => entry(
                "directive",
                Some(d.name),
                d.arguments.len(),
                d.description.clone(),
            ),
        })
        .collect())
}

fn async_graphql_schema(src: &str) -> Result<Vec<Summary>, String> {
    use async_graphql_parser::types::{TypeKind as K, TypeSystemDefinition as D};
    let doc = async_graphql_parser::parse_schema(src).map_err(|e| e.to_string())?;
    Ok(doc
        .definitions
        .iter()
        .map(|d| match d {
            D::Schema(s) => {
                let s = &s.node;
                let roots = [&s.query, &s.mutation, &s.subscription];
                let kind = if s.extend { "extend schema" } else { "schema" };
                entry(
                    kind,
                    None,
                    roots.iter().filter(|r| r.is_some()).count(),
                    None,
                )
            }
            D::Type(t) => {
                let t = &t.node;
                let (kind, members) = match &t.kind {
                    K::Scalar => ("scalar", 0),
                    K::Object(o) => ("type", o.fields.len()),
                    K::Interface(i) => ("interface", i.fields.len()),
                    K::Union(u) => ("union", u.members.len()),
                    K::Enum(e) => ("enum", e.values.len()),
                    K::InputObject(i) => ("input", i.fields.len()),
                };
                let kind = if t.extend {
                    format!("extend {kind}")
                } else {
                    kind.to_string()
                };
                let description = t.description.as_ref().map(|d| d.node.clone());
                entry(&kind, Some(t.name.node.as_str()), members, description)
            }
            D::Directive(d) => {
                let d = &d.node;
                let description = d.description.as_ref().map(|d| d.node.clone());
                entry(
                    "directive",
                    Some(d.name.node.as_str()),
                    d.arguments.len(),
                    description,
                )
            }
        })
        .collect())
}

fn apollo_schema(src: &str) -> Result<Vec<Summary>, String> {
    use apollo_parser::cst::{self, Definition as D};
    let tree = apollo_parser::Parser::new(src).parse();
    if let Some(e) = tree.errors().next() {
        return Err(e.message().to_string());
    }
    let text = |d: Option<cst::Description>| d.and_then(|d| d.string_value()).map(String::from);
    let fields = |f: Option<cst::FieldsDefinition>| f.map_or(0, |f| f.field_definitions().count());
    let members = |u: Option<cst::UnionMemberTypes>| u.map_or(0, |u| u.named_types().count());
    let values =
        |e: Option<cst::EnumValuesDefinition>| e.map_or(0, |e| e.enum_value_definitions().count());
    let inputs = |i: Option<cst::InputFieldsDefinition>| {
        i.map_or(0, |i| i.input_value_definitions().count())
    };
    tree.document()
        .definitions()
        .map(|d| {
            let name = d.name().map(|n| n.text().to_string());
            let (kind, members, description) = match &d {
                D::SchemaDefinition(d) => (
                    "schema",
                    d.root_operation_type_definitions().count(),
                    text(d.description()),
                ),
                D::SchemaExtension(d) => (
                    "extend schema",
                    d.root_operation_type_definitions().count(),
                    None,
                ),
                D::ScalarTypeDefinition(d) => ("scalar", 0, text(d.description())),
                D::ScalarTypeExtension(_) => ("extend scalar", 0, None),
                D::ObjectTypeDefinition(d) => {
                    ("type", fields(d.fields_definition()), text(d.description()))
                }
                D::ObjectTypeExtension(d) => ("extend type", fields(d.fields_definition()), None),
                D::InterfaceTypeDefinition(d) => (
                    "interface",
                    fields(d.fields_definition()),
                    text(d.description()),
                ),
                D::InterfaceTypeExtension(d) => {
                    ("extend interface", fields(d.fields_definition()), None)
                }
                D::UnionTypeDefinition(d) => (
                    "union",
                    members(d.union_member_types()),
                    text(d.description()),
                ),
                D::UnionTypeExtension(d) => ("extend union", members(d.union_member_types()), None),
                D::EnumTypeDefinition(d) => (
                    "enum",
                    values(d.enum_values_definition()),
                    text(d.description()),
                ),
                D::EnumTypeExtension(d) => {
                    ("extend enum", values(d.enum_values_definition()), None)
                }
                D::InputObjectTypeDefinition(d) => (
                    "input",
                    inputs(d.input_fields_definition()),
                    text(d.description()),
                ),
                D::InputObjectTypeExtension(d) => {
                    ("extend input", inputs(d.input_fields_definition()), None)
                }
                D::DirectiveDefinition(d) => {
                    let arguments = d
                        .arguments_definition()
                        .map_or(0, |a| a.input_value_definitions().count());
                    ("directive", arguments, text(d.description()))
                }
                D::OperationDefinition(_) | D::FragmentDefinition(_) => {
                    return Err("an executable definition".to_string());
                }
            };
            Ok(Summary {
                kind: kind.to_string(),
                name,
                members,
                description,
            })
        })
        .collect()
}

fn cynic_schema(src: &str) -> Result<Vec<Summary>, String> {
    use cynic_parser::type_system::{Definition as D, Description, TypeDefinition as T};
    let doc = cynic_parser::parse_type_system_document(src).map_err(|e| e.to_string())?;
    let text = |d: Option<Description<'_>>| d.map(|d| d.to_cow().into_owned());
    let ty = |t: &T<'_>| match t {
        T::Scalar(_) => ("scalar", 0),
        T::Object(o) => ("type", o.fields().len()),
        T::Interface(i) => ("interface", i.fields().len()),
        T::Union(u) => ("union", u.members().len()),
        T::Enum(e) => ("enum", e.values().len()),
        T::InputObject(i) => ("input", i.fields().len()),
    };
    Ok(doc
        .definitions()
        .map(|d| match d {
            D::Schema(s) => entry(
                "schema",
                None,
                s.root_operations().len(),
                text(s.description()),
            ),
            D::SchemaExtension(s) => entry("extend schema", None, s.root_operations().len(), None),
            D::Type(t) => {
                let (kind, members) = ty(&t);
                entry(kind, Some(t.name()), members, text(t.description()))
            }
            D::TypeExtension(t) => {
                let (kind, members) = ty(&t);
                entry(&format!("extend {kind}"), Some(t.name()), members, None)
            }
            D::Directive(d) => entry(
                "directive",
                Some(d.name()),
                d.arguments().len(),
                text(d.description()),
            ),
        })
        .collect())
}

// Executable documents.

fn prpr_query(src: &str) -> Result<Vec<Summary>, String> {
    let doc = prpr_graphql::executable_document(src).map_err(|e| e.to_string())?;
    Ok(graphql::summary(&doc))
}

fn graphql_parser_query(src: &str) -> Result<Vec<Summary>, String> {
    use graphql_parser::query::{Definition as D, OperationDefinition as O};
    let doc = graphql_parser::parse_query::<&str>(src).map_err(|e| e.to_string())?;
    Ok(doc
        .definitions
        .iter()
        .map(|d| match d {
            D::Operation(O::SelectionSet(s)) => entry("query", None, s.items.len(), None),
            D::Operation(O::Query(q)) => entry("query", q.name, q.selection_set.items.len(), None),
            D::Operation(O::Mutation(m)) => {
                entry("mutation", m.name, m.selection_set.items.len(), None)
            }
            D::Operation(O::Subscription(s)) => {
                entry("subscription", s.name, s.selection_set.items.len(), None)
            }
            D::Fragment(f) => entry("fragment", Some(f.name), f.selection_set.items.len(), None),
        })
        .collect())
}

fn async_graphql_query(src: &str) -> Result<Vec<Summary>, String> {
    use async_graphql_parser::types::OperationType;
    let doc = async_graphql_parser::parse_query(src).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (name, op) in doc.operations.iter() {
        let kind = match op.node.ty {
            OperationType::Query => "query",
            OperationType::Mutation => "mutation",
            OperationType::Subscription => "subscription",
        };
        let selections = op.node.selection_set.node.items.len();
        out.push(entry(kind, name.map(|n| n.as_str()), selections, None));
    }
    for (name, f) in &doc.fragments {
        let selections = f.node.selection_set.node.items.len();
        out.push(entry("fragment", Some(name.as_str()), selections, None));
    }
    Ok(out)
}

fn apollo_query(src: &str) -> Result<Vec<Summary>, String> {
    use apollo_parser::cst::Definition as D;
    let tree = apollo_parser::Parser::new(src).parse();
    if let Some(e) = tree.errors().next() {
        return Err(e.message().to_string());
    }
    tree.document()
        .definitions()
        .map(|d| {
            let name = d.name().map(|n| n.text().to_string());
            let (kind, selection_set) = match &d {
                D::OperationDefinition(op) => {
                    let kind = match op.operation_type() {
                        Some(t) if t.mutation_token().is_some() => "mutation",
                        Some(t) if t.subscription_token().is_some() => "subscription",
                        _ => "query",
                    };
                    (kind, op.selection_set())
                }
                D::FragmentDefinition(f) => ("fragment", f.selection_set()),
                _ => return Err("a type system definition".to_string()),
            };
            let members = selection_set.map_or(0, |s| s.selections().count());
            Ok(entry(kind, name.as_deref(), members, None))
        })
        .collect()
}

fn cynic_query(src: &str) -> Result<Vec<Summary>, String> {
    use cynic_parser::executable::ExecutableDefinition as D;
    let doc = cynic_parser::parse_executable_document(src).map_err(|e| e.to_string())?;
    Ok(doc
        .definitions()
        .map(|d| match d {
            D::Operation(op) => {
                let kind = op.operation_type().as_str();
                entry(kind, op.name(), op.selection_set().len(), None)
            }
            D::Fragment(f) => entry("fragment", Some(f.name()), f.selection_set().len(), None),
        })
        .collect())
}

// Checks that the parsers agree with prpr, the first one, on the document at `path`. Executable
// documents are compared in no particular order, since async-graphql-parser does not keep it.
fn agree(path: &str, parsers: &[(&str, Parse)], ordered: bool) {
    let text = std::fs::read_to_string(path).unwrap();
    let sorted = |mut summary: Vec<Summary>| {
        if !ordered {
            summary.sort_by(|a, b| (&a.kind, &a.name).cmp(&(&b.kind, &b.name)));
        }
        summary
    };
    let (_, prpr) = parsers[0];
    let expected = sorted(prpr(&text).unwrap_or_else(|e| panic!("prpr: {path}: {e}")));
    for (name, parse) in &parsers[1..] {
        let actual = sorted(parse(&text).unwrap_or_else(|e| panic!("{name}: {path}: {e}")));
        assert_eq!(actual.len(), expected.len(), "{name} disagrees on {path}");
        for (a, e) in actual.iter().zip(&expected) {
            assert_eq!(a, e, "{name} disagrees on {path}");
        }
    }
}

const SCHEMA_PARSERS: &[(&str, Parse)] = &[
    ("prpr", prpr_schema),
    ("graphql-parser", graphql_parser_schema),
    ("async-graphql-parser", async_graphql_schema),
    ("apollo-parser", apollo_schema),
    ("cynic-parser", cynic_schema),
];

const QUERY_PARSERS: &[(&str, Parse)] = &[
    ("prpr", prpr_query),
    ("graphql-parser", graphql_parser_query),
    ("async-graphql-parser", async_graphql_query),
    ("apollo-parser", apollo_query),
    ("cynic-parser", cynic_query),
];

// The parsers that support all of the type system definitions and executable documents of the
// specification.
fn complete(parsers: &[(&'static str, Parse)]) -> Vec<(&'static str, Parse)> {
    let incomplete = ["graphql-parser", "async-graphql-parser"];
    parsers
        .iter()
        .copied()
        .filter(|(name, _)| !incomplete.contains(name))
        .collect()
}

#[test]
fn schemas() {
    agree("tests/graphql/schema-common.graphql", SCHEMA_PARSERS, true);
    let complete = complete(SCHEMA_PARSERS);
    agree("tests/graphql/schema-full.graphql", &complete, true);
    // The schema of the benchmark, if it has been downloaded.
    let dir = std::env::var("PRPR_BENCH_DATA").unwrap_or_else(|_| "data".into());
    let github = format!("{dir}/github.graphql");
    if std::path::Path::new(&github).exists() {
        agree(&github, SCHEMA_PARSERS, true);
    }
}

#[test]
fn queries() {
    agree("tests/graphql/query-common.graphql", QUERY_PARSERS, false);
    let complete = complete(QUERY_PARSERS);
    agree("tests/graphql/query-full.graphql", &complete, true);
}

// What prpr parses, beyond what the parsers agree on.

#[test]
fn prpr_values() {
    use prpr_benchmarks::graphql::{Definition, Selection, Value};
    let text = std::fs::read_to_string("tests/graphql/query-full.graphql").unwrap();
    let doc = prpr_graphql::parse_query(&text);
    let arguments = |definition: usize, field: &str| {
        let selections = match &doc.definitions[definition] {
            Definition::Operation(op) => &op.selection_set,
            Definition::Fragment(f) => &f.selection_set,
            _ => unreachable!(),
        };
        selections
            .iter()
            .find_map(|s| match s {
                Selection::Field(f) if f.name == field => Some(f.arguments.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        arguments(4, "numbers"),
        [
            ("zero", Value::Int(0)),
            ("float", Value::Float(1.5e-3)),
            ("exp", Value::Float(2e10)),
            ("negative", Value::Float(-1.25)),
        ]
    );
    assert_eq!(
        arguments(4, "enums"),
        [
            ("value", Value::Enum("ENUM_VALUE")),
            ("prefixed", Value::Enum("trueish"))
        ]
    );
    assert_eq!(
        arguments(4, "strings"),
        [(
            "escapes",
            Value::String("\" \\ / \u{8} \u{c} \n \r \t \u{e9}".into())
        )]
    );
    let Value::Object(fields) = &arguments(3, "foo")[2].1 else {
        panic!("not an object");
    };
    assert_eq!(
        fields[1],
        ("block", Value::String("block string uses \"\"\"".into()))
    );
    // Spans.
    let Definition::Operation(op) = &doc.definitions[0] else {
        panic!("not an operation");
    };
    let span = &text[op.span];
    assert!(span.starts_with("query queryName(") && span.ends_with("}\n}"));
    let Selection::Field(node) = &op.selection_set[0] else {
        panic!("not a field");
    };
    assert!(text[node.span].starts_with("whoever123is: node("));
    let Selection::InlineFragment(fragment) = &node.selection_set[1] else {
        panic!("not an inline fragment");
    };
    assert!(text[fragment.span].starts_with("... on User @onInlineFragment {"));
}

#[test]
fn prpr_errors() {
    let error = |s: &str| prpr_graphql::document(s).unwrap_err().to_string();
    assert_eq!(error("{ }"), "expected selection, found `}`");
    assert_eq!(error("query Q($x Int) { a }"), "expected `:`, found `I`");
    assert_eq!(
        error("fragment on on User { a }"),
        "expected fragment name, found `o`"
    );
    assert_eq!(
        error("type T implements { a: Int }"),
        "expected one of `&` or name, found `{`"
    );
    assert_eq!(error("{ a(n: 01) }"), "expected delimiter, found `1`");
    assert_eq!(
        error("{ a(n: 99999999999999999999) }"),
        "expected an integer that fits in 64 bits, found `9`"
    );
    assert_eq!(
        error(r#"{ a(s: "\q") }"#),
        "expected escape sequence, found `q`"
    );
    assert_eq!(
        error(r#"{ a(s: "abc) }"#),
        "expected a string that ends on the same line, found `\"`"
    );
}
