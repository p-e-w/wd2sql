// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2022  Philipp Emanuel Weidmann <pew@worldwidemann.com>

mod id;
mod value;

use std::{
    fs::File,
    io::{stdin, stdout, BufRead, BufReader, Read, Write},
    path::Path,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::Parser;
use humansize::{format_size, DECIMAL};
use humantime::format_duration;
use lazy_static::lazy_static;
use rusqlite::Connection;
use wikidata::{Entity, Lang, Rank, WikiId};

use crate::{
    id::{l_id, p_id, q_id},
    value::{Value, VALUE_TYPES},
};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

lazy_static! {
    static ref ENGLISH: Lang = Lang("en".to_owned());
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    json_file: String,
    sqlite_file: String,
}

fn create_tables(connection: &Connection) -> rusqlite::Result<()> {
    connection
        .execute_batch("CREATE TABLE meta (id INTEGER NOT NULL, label TEXT, description TEXT);")?;

    for value_type in VALUE_TYPES.iter() {
        value_type.create_table(connection)?;
    }

    Ok(())
}

fn create_indices(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE INDEX meta_id_index ON meta (id);
        CREATE INDEX meta_label_index ON meta (label);
        CREATE INDEX meta_description_index ON meta (description);
        ",
    )?;

    for value_type in VALUE_TYPES.iter() {
        value_type.create_indices(connection)?;
    }

    Ok(())
}

fn store_entity(connection: &Connection, entity: Entity) -> rusqlite::Result<()> {
    use WikiId::*;

    let id = match entity.id {
        EntityId(id) => q_id(id),
        PropertyId(id) => p_id(id),
        LexemeId(id) => l_id(id),
    };

    connection
        .prepare_cached("INSERT INTO meta (id, label, description) VALUES (?1, ?2, ?3)")?
        .execute((
            id,
            entity.labels.get(&ENGLISH),
            entity.descriptions.get(&ENGLISH),
        ))?;

    for (pid, claim_value) in entity.claims {
        if claim_value.rank != Rank::Deprecated {
            Value::from(claim_value.data).store(connection, id, p_id(pid))?;
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();

    if Path::new(&arguments.sqlite_file).exists() {
        eprintln!(
            "The database '{}' already exists. Updating an existing database is not supported. Choose a new filename for the database.",
            arguments.sqlite_file,
        );
        return ExitCode::FAILURE;
    }

    let start_time = Instant::now();

    let print_progress = |entity_count, byte_count, finished| {
        print!(
            "\x1B[2K\r{} entities, {} processed in {}{}",
            entity_count,
            format_size(byte_count, DECIMAL),
            format_duration(Duration::new(start_time.elapsed().as_secs(), 0)),
            ".".repeat(if finished { 1 } else { 3 }),
        );

        let _ = stdout().flush();
    };

    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let reader: Box<dyn Read> = if arguments.json_file == "-" {
        Box::new(stdin())
    } else {
        Box::new(match File::open(&arguments.json_file) {
            Ok(file) => file,
            Err(error) => {
                eprintln!(
                    "Error opening JSON file '{}': {}",
                    arguments.json_file, error,
                );
                return ExitCode::FAILURE;
            }
        })
    };

    let reader = BufReader::new(reader);

    let connection = match Connection::open(&arguments.sqlite_file) {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!(
                "Error opening SQLite database '{}': {}",
                arguments.sqlite_file, error,
            );
            return ExitCode::FAILURE;
        }
    };

    if let Err(error) = connection.pragma_update(None, "synchronous", "OFF") {
        eprintln!("Error disabling synchronous mode: {}", error);
        return ExitCode::FAILURE;
    }

    if let Err(error) = connection.pragma_update(None, "journal_mode", "OFF") {
        eprintln!("Error disabling rollback journal: {}", error);
        return ExitCode::FAILURE;
    }

    if let Err(error) = create_tables(&connection) {
        eprintln!("Error creating tables: {}", error);
        return ExitCode::FAILURE;
    }

    if let Err(error) = connection.execute_batch("BEGIN TRANSACTION;") {
        eprintln!("Error starting transaction: {}", error);
        return ExitCode::FAILURE;
    }

    let mut line_number: usize = 0;
    let mut entity_count: usize = 0;
    let mut byte_count: usize = 0;

    for line in reader.lines() {
        line_number += 1;

        let mut line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("\nError reading line {}: {}", line_number, error);
                continue;
            }
        };

        let line_length = line.len();
        byte_count += line_length;

        // Skip array delimiters at beginning and end of dump.
        if line.is_empty() || line == "[" || line == "]" {
            continue;
        }

        // Remove trailing comma.
        if line.ends_with(',') {
            line.truncate(line_length - 1);
        }

        let value = match unsafe { simd_json::from_str(&mut line) } {
            Ok(value) => value,
            Err(error) => {
                eprintln!("\nError parsing JSON at line {}: {}", line_number, error);
                continue;
            }
        };

        let entity = match Entity::from_json(value) {
            Ok(entity) => entity,
            Err(error) => {
                eprintln!(
                    "\nError parsing entity from JSON at line {}: {:?}",
                    line_number, error,
                );
                continue;
            }
        };

        if let Err(error) = store_entity(&connection, entity) {
            eprintln!("\nError storing entity at line {}: {}", line_number, error);
        }

        entity_count += 1;

        if entity_count % 1000 == 0 {
            if let Err(error) = connection.execute_batch(
                "
                END TRANSACTION;
                BEGIN TRANSACTION;
                ",
            ) {
                eprintln!(
                    "\nError committing transaction at line {}: {}",
                    line_number, error,
                );
            }

            print_progress(entity_count, byte_count, false);
        }
    }

    if let Err(error) = connection.execute_batch("END TRANSACTION;") {
        eprintln!("\nError committing transaction: {}", error);
    }

    print_progress(entity_count, byte_count, true);

    println!("\nCreating indices...");

    if let Err(error) = create_indices(&connection) {
        eprintln!("Error creating indices: {}", error);
    }

    println!("Finished.");

    ExitCode::SUCCESS
}
