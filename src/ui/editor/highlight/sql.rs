//! SQL 着色规则
//!
//! @author sky

use super::TokenKind;

pub fn classify(node: &tree_sitter::Node) -> Option<TokenKind> {
    let kind = node.kind();
    // tree-sitter-sequel 可能用小写 node kind（keyword_xxx 前缀）
    if kind.starts_with("keyword") {
        return Some(TokenKind::Keyword);
    }
    let upper = kind.to_uppercase();
    match upper.as_str() {
        // DDL / DML
        "SELECT" | "FROM" | "WHERE" | "INSERT" | "INTO" | "UPDATE" | "DELETE" | "CREATE"
        | "ALTER" | "DROP" | "TABLE" | "INDEX" | "VIEW" | "DATABASE" | "SCHEMA" | "SET"
        | "VALUES" | "AND" | "OR" | "NOT" | "IN" | "IS" | "NULL" | "LIKE" | "BETWEEN" | "JOIN"
        | "INNER" | "OUTER" | "LEFT" | "RIGHT" | "CROSS" | "ON" | "AS" | "ORDER" | "BY" | "ASC"
        | "DESC" | "GROUP" | "HAVING" | "LIMIT" | "OFFSET" | "UNION" | "ALL" | "DISTINCT"
        | "EXISTS" | "CASE" | "WHEN" | "THEN" | "ELSE" | "END" | "IF" | "BEGIN" | "COMMIT"
        | "ROLLBACK" | "TRANSACTION" | "PRIMARY" | "KEY" | "FOREIGN" | "REFERENCES"
        | "CONSTRAINT" | "DEFAULT" | "CHECK" | "UNIQUE" | "CASCADE" | "TRIGGER" | "PROCEDURE"
        | "FUNCTION" | "RETURNS" | "DECLARE" | "CURSOR" => Some(TokenKind::Keyword),
        // SQL 类型
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "FLOAT" | "DOUBLE" | "DECIMAL"
        | "NUMERIC" | "VARCHAR" | "CHAR" | "TEXT" | "BLOB" | "DATE" | "TIME" | "TIMESTAMP"
        | "DATETIME" | "BOOLEAN" | "BOOL" => Some(TokenKind::Type),
        _ => match kind {
            "string" | "single_quoted_string" | "double_quoted_string" | "\"" | "'" => {
                Some(TokenKind::String)
            }
            "number" | "integer" | "float" => Some(TokenKind::Number),
            "comment" | "line_comment" | "block_comment" | "marginalia" => Some(TokenKind::Comment),
            "type" | "column_type" => Some(TokenKind::Type),
            _ => None,
        },
    }
}
