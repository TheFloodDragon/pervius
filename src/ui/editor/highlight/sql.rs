//! SQL 着色规则
//!
//! @author sky

pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    let kind = node.kind().to_uppercase();
    match kind.as_str() {
        // DDL / DML 关键字
        "SELECT" | "FROM" | "WHERE" | "INSERT" | "INTO" | "UPDATE" | "DELETE" | "CREATE"
        | "ALTER" | "DROP" | "TABLE" | "INDEX" | "VIEW" | "DATABASE" | "SCHEMA" | "SET"
        | "VALUES" | "AND" | "OR" | "NOT" | "IN" | "IS" | "NULL" | "LIKE" | "BETWEEN" | "JOIN"
        | "INNER" | "OUTER" | "LEFT" | "RIGHT" | "CROSS" | "ON" | "AS" | "ORDER" | "BY" | "ASC"
        | "DESC" | "GROUP" | "HAVING" | "LIMIT" | "OFFSET" | "UNION" | "ALL" | "DISTINCT"
        | "EXISTS" | "CASE" | "WHEN" | "THEN" | "ELSE" | "END" | "IF" | "BEGIN" | "COMMIT"
        | "ROLLBACK" | "TRANSACTION" | "PRIMARY" | "KEY" | "FOREIGN" | "REFERENCES"
        | "CONSTRAINT" | "DEFAULT" | "CHECK" | "UNIQUE" | "CASCADE" | "TRIGGER" | "PROCEDURE"
        | "FUNCTION" | "RETURNS" | "DECLARE" | "CURSOR" => 1,
        // 类型
        "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" | "FLOAT" | "DOUBLE" | "DECIMAL"
        | "NUMERIC" | "VARCHAR" | "CHAR" | "TEXT" | "BLOB" | "DATE" | "TIME" | "TIMESTAMP"
        | "DATETIME" | "BOOLEAN" | "BOOL" => 3,
        _ => {
            // fallback: tree-sitter-sequel 可能用小写 node kind
            match node.kind() {
                "keyword" | "keyword_select" | "keyword_from" | "keyword_where"
                | "keyword_insert" | "keyword_update" | "keyword_delete" | "keyword_create"
                | "keyword_alter" | "keyword_drop" | "keyword_table" | "keyword_index"
                | "keyword_set" | "keyword_values" | "keyword_and" | "keyword_or"
                | "keyword_not" | "keyword_in" | "keyword_is" | "keyword_null" | "keyword_join"
                | "keyword_on" | "keyword_as" | "keyword_order" | "keyword_by"
                | "keyword_group" | "keyword_having" | "keyword_limit" => 1,
                "string" | "single_quoted_string" | "double_quoted_string" | "\"" | "'" => 2,
                "number" | "integer" | "float" => 4,
                "comment" | "line_comment" | "block_comment" | "marginalia" => 5,
                "type" | "column_type" => 3,
                _ => -1,
            }
        }
    }
}
