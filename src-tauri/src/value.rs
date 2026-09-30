use crate::{CorzaError, Result};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Text,
    Boolean,
    Blob,
}

impl DataType {
    pub fn parse(input: &str) -> Result<Self> {
        match input.to_ascii_uppercase().as_str() {
            "INT" | "INTEGER" => Ok(Self::Integer),
            "REAL" | "FLOAT" | "DOUBLE" => Ok(Self::Real),
            "TEXT" | "STRING" | "VARCHAR" => Ok(Self::Text),
            "BOOL" | "BOOLEAN" => Ok(Self::Boolean),
            "BLOB" | "BYTES" => Ok(Self::Blob),
            other => Err(CorzaError::Schema(format!("unknown type `{other}`"))),
        }
    }

    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::Integer => 1,
            Self::Real => 2,
            Self::Text => 3,
            Self::Boolean => 4,
            Self::Blob => 5,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Result<Self> {
        match tag {
            1 => Ok(Self::Integer),
            2 => Ok(Self::Real),
            3 => Ok(Self::Text),
            4 => Ok(Self::Boolean),
            5 => Ok(Self::Blob),
            _ => Err(CorzaError::Corrupt(format!("invalid type tag {tag}"))),
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
            Self::Text => "TEXT",
            Self::Boolean => "BOOLEAN",
            Self::Blob => "BLOB",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Boolean(bool),
    Blob(Vec<u8>),
}

impl Value {
    pub fn data_type(&self) -> Option<DataType> {
        match self {
            Self::Null => None,
            Self::Integer(_) => Some(DataType::Integer),
            Self::Real(_) => Some(DataType::Real),
            Self::Text(_) => Some(DataType::Text),
            Self::Boolean(_) => Some(DataType::Boolean),
            Self::Blob(_) => Some(DataType::Blob),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::Integer(value) => write!(f, "{value}"),
            Self::Real(value) => write!(f, "{value}"),
            Self::Text(value) => f.write_str(value),
            Self::Boolean(value) => write!(f, "{value}"),
            Self::Blob(value) => {
                f.write_str("0x")?;
                for byte in value {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
        }
    }

    pub fn required(mut self) -> Self {
        self.nullable = false;
        self
    }
}

pub type Row = Vec<Value>;
