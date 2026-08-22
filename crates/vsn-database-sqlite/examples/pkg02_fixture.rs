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
         CREATE TABLE teams(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);
         CREATE TABLE users(
           id INTEGER PRIMARY KEY,
           team_id INTEGER NOT NULL,
           name TEXT NOT NULL,
           email TEXT NOT NULL UNIQUE,
           note TEXT NOT NULL DEFAULT '',
           FOREIGN KEY(team_id) REFERENCES teams(id) ON DELETE RESTRICT
         );
         CREATE INDEX idx_users_name ON users(name);",
    )?;
    conn.execute("INSERT INTO teams(name) VALUES (?1)", ["Platform"])?;
    conn.execute(
        "INSERT INTO users(team_id,name,email,note) VALUES (?1,?2,?3,?4)",
        params![1_i64, "Alice", "alice@example.test", "seed"],
    )?;
    let large = "x".repeat(760 * 1024);
    conn.execute(
        "INSERT INTO users(team_id,name,email,note) VALUES (?1,?2,?3,?4)",
        params![1_i64, "Large", "large@example.test", large],
    )?;
    println!("{}", path.display());
    Ok(())
}
