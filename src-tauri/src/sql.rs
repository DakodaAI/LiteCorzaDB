use crate::{Column, CorzaError, DataType, Result, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    CreateTable {
        name: String,
        columns: Vec<Column>,
    },
    Insert {
        table: String,
        values: Vec<Value>,
    },
    Select {
        table: String,
        columns: Vec<String>,
        filter: Option<(String, Value)>,
        limit: Option<usize>,
    },
    ShowTables,
    Describe {
        table: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    String(String),
    Number(String),
    Symbol(char),
}

pub fn parse(input: &str) -> Result<Statement> {
    let mut parser = Parser {
        tokens: tokenize(input)?,
        cursor: 0,
    };
    let statement = parser.statement()?;
    parser.consume_symbol(';');
    if parser.peek().is_some() {
        return Err(parser.error("unexpected trailing input"));
    }
    Ok(statement)
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else if matches!(ch, '(' | ')' | ',' | ';' | '=' | '*') {
            tokens.push(Token::Symbol(ch));
            chars.next();
        } else if ch == '\'' {
            chars.next();
            let mut value = String::new();
            loop {
                match chars.next() {
                    Some('\'') if chars.peek() == Some(&'\'') => {
                        chars.next();
                        value.push('\'');
                    }
                    Some('\'') => break,
                    Some(c) => value.push(c),
                    None => return Err(CorzaError::Parse("unterminated string literal".into())),
                }
            }
            tokens.push(Token::String(value));
        } else if ch.is_ascii_digit()
            || (ch == '-' && chars.clone().nth(1).is_some_and(|c| c.is_ascii_digit()))
        {
            let mut number = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || matches!(c, '-' | '.' | 'e' | 'E' | '+') {
                    number.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Number(number));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Word(word));
        } else {
            return Err(CorzaError::Parse(format!("unexpected character `{ch}`")));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn statement(&mut self) -> Result<Statement> {
        if self.consume_keyword("CREATE") {
            self.expect_keyword("TABLE")?;
            self.create_table()
        } else if self.consume_keyword("INSERT") {
            self.expect_keyword("INTO")?;
            self.insert()
        } else if self.consume_keyword("SELECT") {
            self.select()
        } else if self.consume_keyword("SHOW") {
            self.expect_keyword("TABLES")?;
            Ok(Statement::ShowTables)
        } else if self.consume_keyword("DESCRIBE") || self.consume_keyword("DESC") {
            Ok(Statement::Describe {
                table: self.identifier()?,
            })
        } else {
            Err(self.error("expected CREATE, INSERT, SELECT, SHOW, or DESCRIBE"))
        }
    }

    fn create_table(&mut self) -> Result<Statement> {
        let name = self.identifier()?;
        self.expect_symbol('(')?;
        let mut columns = Vec::new();
        loop {
            let column_name = self.identifier()?;
            let type_name = self.identifier()?;
            let mut column = Column::new(column_name, DataType::parse(&type_name)?);
            if self.consume_keyword("NOT") {
                self.expect_keyword("NULL")?;
                column.nullable = false;
            }
            columns.push(column);
            if self.consume_symbol(')') {
                break;
            }
            self.expect_symbol(',')?;
        }
        Ok(Statement::CreateTable { name, columns })
    }

    fn insert(&mut self) -> Result<Statement> {
        let table = self.identifier()?;
        self.expect_keyword("VALUES")?;
        self.expect_symbol('(')?;
        let mut values = Vec::new();
        loop {
            values.push(self.value()?);
            if self.consume_symbol(')') {
                break;
            }
            self.expect_symbol(',')?;
        }
        Ok(Statement::Insert { table, values })
    }

    fn select(&mut self) -> Result<Statement> {
        let mut columns = Vec::new();
        if self.consume_symbol('*') {
            columns.push("*".into());
        } else {
            loop {
                columns.push(self.identifier()?);
                if !self.consume_symbol(',') {
                    break;
                }
            }
        }
        self.expect_keyword("FROM")?;
        let table = self.identifier()?;
        let filter = if self.consume_keyword("WHERE") {
            let column = self.identifier()?;
            self.expect_symbol('=')?;
            Some((column, self.value()?))
        } else {
            None
        };
        let limit = if self.consume_keyword("LIMIT") {
            let raw = match self.next() {
                Some(Token::Number(value)) => value,
                _ => return Err(self.error("expected a non-negative LIMIT")),
            };
            Some(
                raw.parse::<usize>()
                    .map_err(|_| self.error("invalid LIMIT"))?,
            )
        } else {
            None
        };
        Ok(Statement::Select {
            table,
            columns,
            filter,
            limit,
        })
    }

    fn value(&mut self) -> Result<Value> {
        match self.next() {
            Some(Token::String(value)) => Ok(Value::Text(value)),
            Some(Token::Number(value)) if value.contains(['.', 'e', 'E']) => value
                .parse()
                .map(Value::Real)
                .map_err(|_| self.error("invalid real number")),
            Some(Token::Number(value)) => value
                .parse()
                .map(Value::Integer)
                .map_err(|_| self.error("invalid integer")),
            Some(Token::Word(word)) if word.eq_ignore_ascii_case("NULL") => Ok(Value::Null),
            Some(Token::Word(word)) if word.eq_ignore_ascii_case("TRUE") => {
                Ok(Value::Boolean(true))
            }
            Some(Token::Word(word)) if word.eq_ignore_ascii_case("FALSE") => {
                Ok(Value::Boolean(false))
            }
            _ => Err(self.error("expected a literal value")),
        }
    }

    fn identifier(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::Word(value)) => Ok(value),
            _ => Err(self.error("expected an identifier")),
        }
    }
    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(self.error(&format!("expected {keyword}")))
        }
    }
    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let yes =
            matches!(self.peek(), Some(Token::Word(word)) if word.eq_ignore_ascii_case(keyword));
        if yes {
            self.cursor += 1;
        }
        yes
    }
    fn expect_symbol(&mut self, symbol: char) -> Result<()> {
        if self.consume_symbol(symbol) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{symbol}`")))
        }
    }
    fn consume_symbol(&mut self, symbol: char) -> bool {
        let yes = self.peek() == Some(&Token::Symbol(symbol));
        if yes {
            self.cursor += 1;
        }
        yes
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }
    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        self.cursor += usize::from(token.is_some());
        token
    }
    fn error(&self, message: &str) -> CorzaError {
        CorzaError::Parse(format!("{message} at token {}", self.cursor + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_select_with_filter_and_limit() {
        assert_eq!(
            parse("SELECT id, name FROM users WHERE id = 7 LIMIT 1;").unwrap(),
            Statement::Select {
                table: "users".into(),
                columns: vec!["id".into(), "name".into()],
                filter: Some(("id".into(), Value::Integer(7))),
                limit: Some(1)
            }
        );
    }
    #[test]
    fn parses_escaped_quote() {
        let statement = parse("INSERT INTO notes VALUES (1, 'Corza''s tiny DB')").unwrap();
        assert!(
            matches!(statement, Statement::Insert { values, .. } if values[1] == Value::Text("Corza's tiny DB".into()))
        );
    }
}
