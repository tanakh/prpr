//! A GraphQL parser written with prpr, in the grammar file `graphql.peg`.

use crate::graphql::Document;

prpr::include_grammar!("graphql.peg");

pub fn parse_schema(s: &str) -> Document<'_> {
    type_system_document(s).unwrap()
}

pub fn parse_query(s: &str) -> Document<'_> {
    executable_document(s).unwrap()
}
