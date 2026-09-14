use crate::api::FunctionDeclaration;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Column, Row};

pub struct MysqlListTablesTool {
    connection_url: String,
}

impl MysqlListTablesTool {
    pub fn new(connection_url: Option<String>) -> Self {
        Self {
            connection_url: connection_url.unwrap_or_else(|| "mysql://root:@localhost:3306/test".to_string()),
        }
    }
}

#[async_trait]
impl AgentTool for MysqlListTablesTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "mysql_list_tables".to_string(),
            description: "List all tables in the XAMPP MySQL database.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {},
                "required": []
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, _args: Value) -> String {
        let pool = match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&self.connection_url)
            .await
        {
            Ok(p) => p,
            Err(e) => return format!("MySQL Connection Error: {e} (Ensure XAMPP MySQL is running on port 3306)"),
        };

        let query = "SHOW TABLES;";
        let rows: Result<Vec<sqlx::mysql::MySqlRow>, _> = sqlx::query(query).fetch_all(&pool).await;

        match rows {
            Ok(result_rows) => {
                let mut tables = Vec::new();
                for row in result_rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        tables.push(table_name);
                    }
                }
                serde_json::to_string_pretty(&tables).unwrap_or_else(|_| "[]".to_string())
            }
            Err(e) => format!("MySQL Query Error: {e}"),
        }
    }
}

pub struct MysqlExecuteQueryTool {
    connection_url: String,
}

impl MysqlExecuteQueryTool {
    pub fn new(connection_url: Option<String>) -> Self {
        Self {
            connection_url: connection_url.unwrap_or_else(|| "mysql://root:@localhost:3306/test".to_string()),
        }
    }
}

#[async_trait]
impl AgentTool for MysqlExecuteQueryTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "mysql_execute_query".to_string(),
            description: "Execute a SQL query against the XAMPP MySQL database (SELECT, DESCRIBE, SHOW, etc.).".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "query": { "type": "STRING", "description": "SQL query to execute" }
                },
                "required": ["query"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> String {
        let sql = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return "Error: Missing required argument 'query'".to_string(),
        };

        let pool = match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&self.connection_url)
            .await
        {
            Ok(p) => p,
            Err(e) => return format!("MySQL Connection Error: {e}"),
        };

        let result = sqlx::query(sql).fetch_all(&pool).await;
        match result {
            Ok(rows) => {
                let mut output = Vec::new();
                for row in rows {
                    let columns = row.columns();
                    let mut map = serde_json::Map::new();
                    for col in columns {
                        let col_name = col.name();
                        // Try parsing as string, fallback to debug or null
                        let val: Result<String, _> = row.try_get(col_name);
                        match val {
                            Ok(v) => { map.insert(col_name.to_string(), Value::String(v)); }
                            Err(_) => {
                                let int_val: Result<i64, _> = row.try_get(col_name);
                                match int_val {
                                    Ok(i) => { map.insert(col_name.to_string(), Value::Number(i.into())); }
                                    Err(_) => { map.insert(col_name.to_string(), Value::Null); }
                                }
                            }
                        }
                    }
                    output.push(Value::Object(map));
                }
                serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string())
            }
            Err(e) => {
                // Try execute for non-SELECT statements (INSERT, UPDATE, DELETE, CREATE)
                match sqlx::query(sql).execute(&pool).await {
                    Ok(res) => format!("Query executed successfully. Rows affected: {}", res.rows_affected()),
                    Err(exec_err) => format!("MySQL Execution Error: {e} | Exec Error: {exec_err}"),
                }
            }
        }
    }
}

pub struct MysqlDescribeTableTool {
    connection_url: String,
}

impl MysqlDescribeTableTool {
    pub fn new(connection_url: Option<String>) -> Self {
        Self {
            connection_url: connection_url.unwrap_or_else(|| "mysql://root:@localhost:3306/test".to_string()),
        }
    }
}

#[async_trait]
impl AgentTool for MysqlDescribeTableTool {
    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "mysql_describe_table".to_string(),
            description: "Describe table schema, columns, types, and keys for a MySQL table.".to_string(),
            parameters: json!({
                "type": "OBJECT",
                "properties": {
                    "tableName": { "type": "STRING", "description": "Name of the table to describe" }
                },
                "required": ["tableName"]
            }),
        }
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> String {
        let table_name = match args.get("tableName").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return "Error: Missing required argument 'tableName'".to_string(),
        };

        let pool = match MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&self.connection_url)
            .await
        {
            Ok(p) => p,
            Err(e) => return format!("MySQL Connection Error: {e}"),
        };

        let query = format!("DESCRIBE `{}`;", table_name.replace('`', ""));
        let result = sqlx::query(&query).fetch_all(&pool).await;

        match result {
            Ok(rows) => {
                let mut output = Vec::new();
                for row in rows {
                    let columns = row.columns();
                    let mut map = serde_json::Map::new();
                    for col in columns {
                        let col_name = col.name();
                        let val: Result<String, _> = row.try_get(col_name);
                        if let Ok(v) = val {
                            map.insert(col_name.to_string(), Value::String(v));
                        }
                    }
                    output.push(Value::Object(map));
                }
                serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_string())
            }
            Err(e) => format!("MySQL Describe Error: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mysql_tools_offline() {
        // Test offline execution behavior when XAMPP MySQL is not running
        let list_tool = MysqlListTablesTool::new(Some("mysql://root:@localhost:3306/nonexistent".to_string()));
        let res = list_tool.execute(json!({})).await;
        assert!(res.contains("MySQL Connection Error"));

        let query_tool = MysqlExecuteQueryTool::new(Some("mysql://root:@localhost:3306/nonexistent".to_string()));
        let res2 = query_tool.execute(json!({ "query": "SELECT 1" })).await;
        assert!(res2.contains("MySQL Connection Error"));
    }
}
