use lite_corzadb::{Database, QueryResult, Result, Value};
use std::{
    env,
    io::{self, BufRead, Write},
    path::PathBuf,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = PathBuf::from(args.next().unwrap_or_else(|| "corza.db".into()));
    let mut db = Database::open(&path)?;
    if let Some(sql) = args.next() {
        print_result(&db.execute(&sql)?);
        return Ok(());
    }

    println!(
        "Lite CorzaDB {} · {}",
        lite_corzadb::VERSION,
        path.display()
    );
    println!("Enter SQL, `.stats`, or `.quit`.");
    let stdin = io::stdin();
    loop {
        print!("corza> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        match line.trim() {
            "" => continue,
            ".quit" | ".exit" => break,
            ".stats" => {
                let stats = db.stats();
                println!(
                    "{} tables · {} rows · {} bytes",
                    stats.table_count, stats.row_count, stats.file_bytes
                );
            }
            sql => match db.execute(sql) {
                Ok(result) => print_result(&result),
                Err(error) => eprintln!("error: {error}"),
            },
        }
    }
    Ok(())
}

fn print_result(result: &QueryResult) {
    if result.columns.is_empty() {
        println!("{}", result.message);
        return;
    }
    println!("{}", result.columns.join(" | "));
    println!(
        "{}",
        result
            .columns
            .iter()
            .map(|name| "-".repeat(name.len().max(3)))
            .collect::<Vec<_>>()
            .join("-+-")
    );
    for row in &result.rows {
        println!(
            "{}",
            row.iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    println!("{}", result.message);
}

fn display_value(value: &Value) -> String {
    value.to_string()
}
