use crate::{database::Table, Column, CorzaError, DataType, Database, Result, Row, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

const MAGIC: &[u8; 8] = b"CORZADB\0";
const FORMAT_VERSION: u16 = 1;
const HEADER_LEN: usize = 26;
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_VALUE_BYTES: usize = 16 * 1024 * 1024;

pub(crate) fn save(database: &Database, path: &Path) -> Result<()> {
    let mut payload = Vec::new();
    write_u32(&mut payload, database.tables.len())?;
    for (name, table) in &database.tables {
        write_string(&mut payload, name)?;
        write_u32(&mut payload, table.columns.len())?;
        for column in &table.columns {
            write_string(&mut payload, &column.name)?;
            payload.push(column.data_type.tag());
            payload.push(u8::from(column.nullable));
        }
        payload.extend_from_slice(&(table.rows.len() as u64).to_le_bytes());
        for row in &table.rows {
            for value in row {
                write_value(&mut payload, value)?;
            }
        }
    }
    if payload.len() as u64 > MAX_FILE_BYTES {
        return Err(CorzaError::Limit("database exceeds 4 GiB MVP limit".into()));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("corza.tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(MAGIC)?;
    file.write_all(&FORMAT_VERSION.to_le_bytes())?;
    file.write_all(&(payload.len() as u64).to_le_bytes())?;
    file.write_all(&checksum(&payload).to_le_bytes())?;
    file.write_all(&payload)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    sync_parent(path)?;
    Ok(())
}

pub(crate) fn load(path: &Path) -> Result<Database> {
    let metadata = path.metadata()?;
    if metadata.len() < HEADER_LEN as u64 || metadata.len() > MAX_FILE_BYTES + HEADER_LEN as u64 {
        return Err(CorzaError::Corrupt("invalid file size".into()));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes.get(..8) != Some(MAGIC) {
        return Err(CorzaError::Corrupt("invalid magic bytes".into()));
    }
    let version = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    if version != FORMAT_VERSION {
        return Err(CorzaError::Corrupt(format!(
            "unsupported format version {version}"
        )));
    }
    let payload_len = u64::from_le_bytes(bytes[10..18].try_into().unwrap()) as usize;
    let expected = u64::from_le_bytes(bytes[18..26].try_into().unwrap());
    let payload = bytes
        .get(HEADER_LEN..)
        .filter(|p| p.len() == payload_len)
        .ok_or_else(|| CorzaError::Corrupt("payload length mismatch".into()))?;
    if checksum(payload) != expected {
        return Err(CorzaError::Corrupt("checksum mismatch".into()));
    }

    let mut reader = Reader::new(payload);
    let table_count = reader.u32()? as usize;
    if table_count > 1_000_000 {
        return Err(CorzaError::Corrupt("unreasonable table count".into()));
    }
    let mut tables = BTreeMap::new();
    for _ in 0..table_count {
        let name = reader.string()?;
        let column_count = reader.u32()? as usize;
        if column_count == 0 || column_count > super::database::MAX_COLUMNS {
            return Err(CorzaError::Corrupt("invalid column count".into()));
        }
        let mut columns = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            columns.push(Column {
                name: reader.string()?,
                data_type: DataType::from_tag(reader.u8()?)?,
                nullable: reader.u8()? != 0,
            });
        }
        let row_count = reader.u64()? as usize;
        if row_count > super::database::MAX_ROWS {
            return Err(CorzaError::Corrupt("unreasonable row count".into()));
        }
        let mut rows = Vec::with_capacity(row_count.min(1024));
        for _ in 0..row_count {
            let mut row = Row::with_capacity(column_count);
            for _ in 0..column_count {
                row.push(reader.value()?);
            }
            rows.push(row);
        }
        if tables.insert(name, Table { columns, rows }).is_some() {
            return Err(CorzaError::Corrupt("duplicate table name".into()));
        }
    }
    if !reader.is_empty() {
        return Err(CorzaError::Corrupt("trailing payload bytes".into()));
    }
    Ok(Database {
        path: Some(path.to_path_buf()),
        tables,
    })
}

fn write_value(target: &mut Vec<u8>, value: &Value) -> Result<()> {
    match value {
        Value::Null => target.push(0),
        Value::Integer(v) => {
            target.push(1);
            target.extend_from_slice(&v.to_le_bytes());
        }
        Value::Real(v) => {
            target.push(2);
            target.extend_from_slice(&v.to_le_bytes());
        }
        Value::Text(v) => {
            target.push(3);
            write_bytes(target, v.as_bytes())?;
        }
        Value::Boolean(v) => {
            target.push(4);
            target.push(u8::from(*v));
        }
        Value::Blob(v) => {
            target.push(5);
            write_bytes(target, v)?;
        }
    }
    Ok(())
}

fn write_string(target: &mut Vec<u8>, value: &str) -> Result<()> {
    write_bytes(target, value.as_bytes())
}
fn write_bytes(target: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    write_u32(target, value.len())?;
    target.extend_from_slice(value);
    Ok(())
}
fn write_u32(target: &mut Vec<u8>, value: usize) -> Result<()> {
    let value = u32::try_from(value)
        .map_err(|_| CorzaError::Limit("value exceeds 32-bit storage limit".into()))?;
    target.extend_from_slice(&value.to_le_bytes());
    Ok(())
}
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<()> {
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| CorzaError::Corrupt("offset overflow".into()))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| CorzaError::Corrupt("unexpected end of file".into()))?;
        self.cursor = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<String> {
        String::from_utf8(self.bytes()?)
            .map_err(|_| CorzaError::Corrupt("invalid UTF-8 string".into()))
    }
    fn bytes(&mut self) -> Result<Vec<u8>> {
        let length = self.u32()? as usize;
        if length > MAX_VALUE_BYTES {
            return Err(CorzaError::Corrupt("value exceeds 16 MiB limit".into()));
        }
        Ok(self.take(length)?.to_vec())
    }
    fn value(&mut self) -> Result<Value> {
        match self.u8()? {
            0 => Ok(Value::Null),
            1 => Ok(Value::Integer(i64::from_le_bytes(
                self.take(8)?.try_into().unwrap(),
            ))),
            2 => Ok(Value::Real(f64::from_le_bytes(
                self.take(8)?.try_into().unwrap(),
            ))),
            3 => self.string().map(Value::Text),
            4 => match self.u8()? {
                0 => Ok(Value::Boolean(false)),
                1 => Ok(Value::Boolean(true)),
                _ => Err(CorzaError::Corrupt("invalid boolean".into())),
            },
            5 => self.bytes().map(Value::Blob),
            tag => Err(CorzaError::Corrupt(format!("invalid value tag {tag}"))),
        }
    }
    fn is_empty(&self) -> bool {
        self.cursor == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn temp_file(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("corzadb-{name}-{}-{nonce}.db", std::process::id()))
    }
    #[test]
    fn persists_and_reopens_database() {
        let path = temp_file("roundtrip");
        let mut db = Database::open(&path).unwrap();
        db.execute("CREATE TABLE metrics (id INTEGER NOT NULL, value REAL)")
            .unwrap();
        db.execute("INSERT INTO metrics VALUES (1, 4.25)").unwrap();
        drop(db);
        let mut reopened = Database::open(&path).unwrap();
        assert_eq!(
            reopened.execute("SELECT * FROM metrics").unwrap().rows[0],
            vec![Value::Integer(1), Value::Real(4.25)]
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn rejects_checksum_corruption() {
        let path = temp_file("corrupt");
        let mut db = Database::open(&path).unwrap();
        db.execute("CREATE TABLE values (id INTEGER)").unwrap();
        let mut bytes = fs::read(&path).unwrap();
        *bytes.last_mut().unwrap() ^= 0xff;
        fs::write(&path, bytes).unwrap();
        assert!(matches!(Database::open(&path), Err(CorzaError::Corrupt(_))));
        let _ = fs::remove_file(path);
    }
}
