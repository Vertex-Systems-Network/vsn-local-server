use rusqlite::{params, Connection};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env::args().nth(1).ok_or("usage: pkg02_fixture <path>")?);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        fs::remove_file(&path)?;
    }

    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         CREATE TABLE teams(
           id INTEGER PRIMARY KEY,
           name TEXT NOT NULL UNIQUE
         );
         CREATE TABLE users(
           id INTEGER PRIMARY KEY,
           team_id INTEGER NOT NULL,
           name TEXT NOT NULL,
           email TEXT NOT NULL UNIQUE,
           note TEXT NOT NULL DEFAULT '',
           FOREIGN KEY(team_id) REFERENCES teams(id) ON DELETE RESTRICT
         );
         CREATE INDEX idx_users_name ON users(name);
         CREATE TABLE bulk(
           id INTEGER PRIMARY KEY,
           payload TEXT NOT NULL
         );",
    )?;

    conn.execute("INSERT INTO teams(name) VALUES (?1)", ["Platform"])?;
    for (name, email, note) in [
        ("Alice", "alice@example.test", "seed-alice"),
        ("Charlie", "charlie@example.test", "seed-charlie"),
    ] {
        conn.execute(
            "INSERT INTO users(team_id,name,email,note) VALUES (?1,?2,?3,?4)",
            params![1_i64, name, email, note],
        )?;
    }

    let oversized = "x".repeat(300 * 1024);
    conn.execute(
        "INSERT INTO users(team_id,name,email,note) VALUES (?1,?2,?3,?4)",
        params![1_i64, "Large", "large@example.test", oversized],
    )?;

    let bulk = "y".repeat(100 * 1024);
    for _ in 0..6 {
        conn.execute("INSERT INTO bulk(payload) VALUES (?1)", [&bulk])?;
    }

    println!("{}", path.display());
    Ok(())
}
