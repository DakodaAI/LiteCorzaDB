use lite_corzadb::Database;
use std::time::Instant;

fn main() {
    const ROWS: usize = 10_000;
    let mut db = Database::memory();
    db.execute("CREATE TABLE events (id INTEGER NOT NULL, kind TEXT, active BOOLEAN)")
        .unwrap();

    let insert_start = Instant::now();
    for id in 0..ROWS {
        db.execute(&format!(
            "INSERT INTO events VALUES ({id}, 'page_view', true)"
        ))
        .unwrap();
    }
    let insert_elapsed = insert_start.elapsed();

    let select_start = Instant::now();
    let result = db
        .execute("SELECT id FROM events WHERE active = true LIMIT 10000")
        .unwrap();
    let select_elapsed = select_start.elapsed();

    println!("rows={ROWS}");
    println!("insert_total_us={}", insert_elapsed.as_micros());
    println!(
        "insert_rows_per_second={:.0}",
        ROWS as f64 / insert_elapsed.as_secs_f64()
    );
    println!("select_total_us={}", select_elapsed.as_micros());
    println!("selected_rows={}", result.rows.len());
}
