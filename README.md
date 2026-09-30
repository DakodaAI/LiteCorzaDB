# Lite CorzaDB

Lite CorzaDB is an experimental embedded database written in Rust. Its direction is simple: a useful SQL workflow, predictable resource use, a tiny dependency surface, and a core that remains understandable enough to audit.

> **Status:** early engine prototype (`0.1.0`). It is usable for experimentation, but it is not yet a replacement for SQLite in production.

## What works now

- Zero third-party Rust dependencies
- Typed tables: `INTEGER`, `REAL`, `TEXT`, `BOOLEAN`, and `BLOB`
- `CREATE TABLE`, `INSERT INTO ... VALUES`, `SELECT`, equality filters, and `LIMIT`
- `SHOW TABLES` and `DESCRIBE`
- Checksummed binary database files
- Atomic write → sync → rename persistence
- Bounded decoding and corruption rejection
- In-memory databases and an interactive CLI
- React query-workspace prototype for the future Tauri desktop shell

## Build and use

```sh
cargo build --release --manifest-path src-tauri/Cargo.toml
./src-tauri/target/release/lite-corzadb demo.db
```

Then try:

```sql
CREATE TABLE users (id INTEGER NOT NULL, name TEXT, active BOOLEAN);
INSERT INTO users VALUES (1, 'Ada', true);
SELECT id, name FROM users WHERE active = true LIMIT 10;
SHOW TABLES;
DESCRIBE users;
```

Run one statement without entering the shell:

```sh
./src-tauri/target/release/lite-corzadb demo.db "SELECT * FROM users"
```

## Validate

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo run --release --manifest-path src-tauri/Cargo.toml --example bench
```

The benchmark is intentionally dependency-free and reports a reproducible local baseline. It is not presented as a SQLite comparison.

## Design constraints

1. Measure before optimizing.
2. Keep the core dependency-free until a dependency earns its footprint.
3. Prefer bounded allocations and explicit file-format limits.
4. Use safe Rust in the storage and query path unless measured evidence justifies otherwise.
5. Never market an unmeasured advantage over SQLite.

## Current storage format

Each file begins with `CORZADB\0`, a format version, payload length, and FNV-1a checksum. The payload stores schemas and typed rows in a compact little-endian encoding. Mutations currently rewrite the complete database atomically; this is deliberately simple and durable for the prototype, but it limits write scalability.

## Roadmap

- Slotted 4 KiB pages and a pager cache
- Write-ahead log with crash-recovery tests
- B+ tree primary and secondary indexes
- Transactions and MVCC experiments
- Broader SQL grammar and query planning
- Differential correctness tests against SQLite
- Criterion-style benchmark suite with memory and CPU counters
- Tauri command bridge from the included React workspace

## Repository layout

```text
src-tauri/src/
  codec.rs       checksummed binary format and atomic persistence
  database.rs    schema, execution, validation, and query results
  sql.rs         dependency-free tokenizer and parser
  value.rs       data types, values, columns, and rows
  main.rs        CLI shell
src/
  main.jsx       desktop query-workspace prototype
```
