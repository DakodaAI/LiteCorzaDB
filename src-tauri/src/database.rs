use crate::{codec, parse, Column, CorzaError, Result, Row, Statement, Value};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

pub const MAX_COLUMNS: usize = 1_024;
pub const MAX_ROWS: usize = 10_000_000;

#[derive(Debug, Clone)]
pub(crate) struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Default)]
pub struct Database {
    pub(crate) path: Option<PathBuf>,
    pub(crate) tables: BTreeMap<String, Table>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    pub rows_affected: usize,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseStats {
    pub table_count: usize,
    pub row_count: usize,
    pub file_bytes: u64,
}

impl Database {
    pub fn memory() -> Self {
        Self::default()
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            codec::load(&path)
        } else {
            Ok(Self {
                path: Some(path),
                tables: BTreeMap::new(),
            })
        }
    }

    pub fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        self.execute_statement(parse(sql)?)
    }

    pub fn execute_statement(&mut self, statement: Statement) -> Result<QueryResult> {
        match statement {
            Statement::CreateTable { name, columns } => {
                self.create_table(&name, columns)?;
                self.flush()?;
                Ok(QueryResult::message(format!("table `{name}` created")))
            }
            Statement::Insert { table, values } => {
                self.insert(&table, values)?;
                self.flush()?;
                Ok(QueryResult {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 1,
                    message: "1 row inserted".into(),
                })
            }
            Statement::Select {
                table,
                columns,
                filter,
                limit,
            } => self.select(&table, &columns, filter.as_ref(), limit),
            Statement::ShowTables => Ok(QueryResult {
                columns: vec!["table".into(), "rows".into()],
                rows: self
                    .tables
                    .iter()
                    .map(|(name, table)| {
                        vec![
                            Value::Text(name.clone()),
                            Value::Integer(table.rows.len() as i64),
                        ]
                    })
                    .collect(),
                rows_affected: 0,
                message: format!("{} tables", self.tables.len()),
            }),
            Statement::Describe { table } => self.describe(&table),
        }
    }

    pub fn flush(&self) -> Result<()> {
        if let Some(path) = &self.path {
            codec::save(self, path)?;
        }
        Ok(())
    }

    pub fn stats(&self) -> DatabaseStats {
        DatabaseStats {
            table_count: self.tables.len(),
            row_count: self.tables.values().map(|table| table.rows.len()).sum(),
            file_bytes: self
                .path
                .as_ref()
                .and_then(|path| path.metadata().ok())
                .map_or(0, |m| m.len()),
        }
    }

    fn create_table(&mut self, name: &str, columns: Vec<Column>) -> Result<()> {
        validate_identifier(name)?;
        if columns.is_empty() || columns.len() > MAX_COLUMNS {
            return Err(CorzaError::Limit(format!(
                "table column count must be 1..={MAX_COLUMNS}"
            )));
        }
        if self.tables.contains_key(name) {
            return Err(CorzaError::Schema(format!("table `{name}` already exists")));
        }
        let mut names = HashSet::with_capacity(columns.len());
        for column in &columns {
            validate_identifier(&column.name)?;
            if !names.insert(column.name.to_ascii_lowercase()) {
                return Err(CorzaError::Schema(format!(
                    "duplicate column `{}`",
                    column.name
                )));
            }
        }
        self.tables.insert(
            name.into(),
            Table {
                columns,
                rows: Vec::new(),
            },
        );
        Ok(())
    }

    fn insert(&mut self, table_name: &str, values: Row) -> Result<()> {
        let table = self
            .tables
            .get_mut(table_name)
            .ok_or_else(|| CorzaError::NotFound(format!("table `{table_name}`")))?;
        if table.rows.len() >= MAX_ROWS {
            return Err(CorzaError::Limit(format!("table exceeds {MAX_ROWS} rows")));
        }
        if values.len() != table.columns.len() {
            return Err(CorzaError::Schema(format!(
                "expected {} values, received {}",
                table.columns.len(),
                values.len()
            )));
        }
        for (column, value) in table.columns.iter().zip(&values) {
            if matches!(value, Value::Null) {
                if !column.nullable {
                    return Err(CorzaError::Schema(format!(
                        "column `{}` cannot be NULL",
                        column.name
                    )));
                }
            } else if value.data_type() != Some(column.data_type) {
                return Err(CorzaError::Schema(format!(
                    "column `{}` expects {}, received {}",
                    column.name,
                    column.data_type,
                    value.data_type().unwrap()
                )));
            }
        }
        table.rows.push(values);
        Ok(())
    }

    fn select(
        &self,
        table_name: &str,
        requested: &[String],
        filter: Option<&(String, Value)>,
        limit: Option<usize>,
    ) -> Result<QueryResult> {
        let table = self
            .tables
            .get(table_name)
            .ok_or_else(|| CorzaError::NotFound(format!("table `{table_name}`")))?;
        let selected: Vec<usize> = if requested == ["*"] {
            (0..table.columns.len()).collect()
        } else {
            requested
                .iter()
                .map(|name| column_index(&table.columns, name))
                .collect::<Result<_>>()?
        };
        let filter = match filter {
            Some((name, value)) => Some((column_index(&table.columns, name)?, value)),
            None => None,
        };
        let rows = table
            .rows
            .iter()
            .filter(|row| {
                filter
                    .as_ref()
                    .is_none_or(|(index, value)| row[*index] == **value)
            })
            .take(limit.unwrap_or(usize::MAX))
            .map(|row| selected.iter().map(|&index| row[index].clone()).collect())
            .collect::<Vec<Row>>();
        Ok(QueryResult {
            columns: selected
                .iter()
                .map(|&index| table.columns[index].name.clone())
                .collect(),
            message: format!("{} rows", rows.len()),
            rows,
            rows_affected: 0,
        })
    }

    fn describe(&self, table_name: &str) -> Result<QueryResult> {
        let table = self
            .tables
            .get(table_name)
            .ok_or_else(|| CorzaError::NotFound(format!("table `{table_name}`")))?;
        Ok(QueryResult {
            columns: vec!["column".into(), "type".into(), "nullable".into()],
            rows: table
                .columns
                .iter()
                .map(|c| {
                    vec![
                        Value::Text(c.name.clone()),
                        Value::Text(c.data_type.to_string()),
                        Value::Boolean(c.nullable),
                    ]
                })
                .collect(),
            rows_affected: 0,
            message: format!("{} columns", table.columns.len()),
        })
    }
}

impl QueryResult {
    fn message(message: String) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
            message,
        }
    }
}

fn column_index(columns: &[Column], name: &str) -> Result<usize> {
    columns
        .iter()
        .position(|column| column.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| CorzaError::NotFound(format!("column `{name}`")))
}

fn validate_identifier(value: &str) -> Result<()> {
    let mut chars = value.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !valid || value.len() > 128 {
        return Err(CorzaError::Schema(format!("invalid identifier `{value}`")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executes_core_sql_flow() {
        let mut db = Database::memory();
        db.execute("CREATE TABLE users (id INTEGER NOT NULL, name TEXT, active BOOLEAN)")
            .unwrap();
        db.execute("INSERT INTO users VALUES (1, 'Ada', true)")
            .unwrap();
        db.execute("INSERT INTO users VALUES (2, 'Linus', false)")
            .unwrap();
        let result = db
            .execute("SELECT name FROM users WHERE active = true LIMIT 1")
            .unwrap();
        assert_eq!(result.rows, vec![vec![Value::Text("Ada".into())]]);
    }
    #[test]
    fn rejects_wrong_value_type() {
        let mut db = Database::memory();
        db.execute("CREATE TABLE things (id INTEGER)").unwrap();
        assert!(matches!(
            db.execute("INSERT INTO things VALUES ('not-an-int')"),
            Err(CorzaError::Schema(_))
        ));
    }
}
