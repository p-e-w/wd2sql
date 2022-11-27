// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2022  Philipp Emanuel Weidmann <pew@worldwidemann.com>

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use rusqlite::{Connection, Params};
use wikidata::ClaimValueData;

use crate::{
    id::{f_id, l_id, p_id, q_id, s_id},
    ENGLISH,
};

lazy_static! {
    pub static ref VALUE_TYPES: Vec<Value> = vec![
        Value::String(String::new()),
        Value::Entity(0),
        Value::Coordinates {
            latitude: 0.0,
            longitude: 0.0,
            precision: 0.0,
            globe_id: 0,
        },
        Value::Quantity {
            amount: 0.0,
            lower_bound: None,
            upper_bound: None,
            unit_id: None,
        },
        Value::Time {
            time: Default::default(),
            precision: 0,
        },
        Value::None,
        Value::Unknown,
    ];
}

pub enum Value {
    String(String),
    Entity(u64),
    Coordinates {
        latitude: f64,
        longitude: f64,
        precision: f64,
        globe_id: u64,
    },
    Quantity {
        amount: f64,
        lower_bound: Option<f64>,
        upper_bound: Option<f64>,
        unit_id: Option<u64>,
    },
    Time {
        time: DateTime<Utc>,
        precision: u8,
    },
    None,
    Unknown,
}

impl Value {
    fn table_definition(&self) -> (String, Vec<(String, String)>) {
        use Value::*;

        let (table_name, mut value_columns) = match self {
            String(_) => (
                "string".to_owned(),
                vec![("string".to_owned(), "TEXT NOT NULL".to_owned())],
            ),
            Entity(_) => (
                "entity".to_owned(),
                vec![("entity_id".to_owned(), "INTEGER NOT NULL".to_owned())],
            ),
            Coordinates { .. } => (
                "coordinates".to_owned(),
                vec![
                    ("latitude".to_owned(), "REAL NOT NULL".to_owned()),
                    ("longitude".to_owned(), "REAL NOT NULL".to_owned()),
                    ("precision".to_owned(), "REAL NOT NULL".to_owned()),
                    ("globe_id".to_owned(), "INTEGER NOT NULL".to_owned()),
                ],
            ),
            Quantity { .. } => (
                "quantity".to_owned(),
                vec![
                    ("amount".to_owned(), "REAL NOT NULL".to_owned()),
                    ("lower_bound".to_owned(), "REAL".to_owned()),
                    ("upper_bound".to_owned(), "REAL".to_owned()),
                    ("unit_id".to_owned(), "INTEGER".to_owned()),
                ],
            ),
            Time { .. } => (
                "time".to_owned(),
                vec![
                    ("time".to_owned(), "DATETIME NOT NULL".to_owned()),
                    ("precision".to_owned(), "INTEGER NOT NULL".to_owned()),
                ],
            ),
            None => ("none".to_owned(), vec![]),
            Unknown => ("unknown".to_owned(), vec![]),
        };

        let mut columns = vec![
            ("id".to_owned(), "INTEGER NOT NULL".to_owned()),
            ("property_id".to_owned(), "INTEGER NOT NULL".to_owned()),
        ];

        columns.append(&mut value_columns);

        (table_name, columns)
    }

    pub fn create_table(&self, connection: &Connection) -> rusqlite::Result<()> {
        let (table_name, columns) = self.table_definition();

        connection.execute_batch(&format!(
            "CREATE TABLE {} ({});",
            table_name,
            columns
                .iter()
                .map(|(column_name, column_type)| format!("{} {}", column_name, column_type))
                .collect::<Vec<_>>()
                .join(", "),
        ))
    }

    pub fn create_indices(&self, connection: &Connection) -> rusqlite::Result<()> {
        let (table_name, columns) = self.table_definition();

        for (column_name, _) in columns {
            connection.execute_batch(&format!(
                "CREATE INDEX {}_{}_index ON {} ({});",
                table_name, column_name, table_name, column_name,
            ))?;
        }

        Ok(())
    }

    fn store_params(&self, connection: &Connection, params: impl Params) -> rusqlite::Result<()> {
        let (table_name, columns) = self.table_definition();

        connection
            .prepare_cached(&format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns
                    .iter()
                    .map(|(column_name, _)| column_name.to_owned())
                    .collect::<Vec<_>>()
                    .join(", "),
                (0..columns.len())
                    .map(|i| format!("?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", "),
            ))?
            .execute(params)?;

        Ok(())
    }

    pub fn store(
        &self,
        connection: &Connection,
        id: u64,
        property_id: u64,
    ) -> rusqlite::Result<()> {
        use Value::*;

        match self {
            String(string) => self.store_params(connection, (id, property_id, string)),
            Entity(entity_id) => self.store_params(connection, (id, property_id, entity_id)),
            Coordinates {
                latitude,
                longitude,
                precision,
                globe_id,
            } => self.store_params(
                connection,
                (id, property_id, latitude, longitude, precision, globe_id),
            ),
            Quantity {
                amount,
                lower_bound,
                upper_bound,
                unit_id,
            } => self.store_params(
                connection,
                (id, property_id, amount, lower_bound, upper_bound, unit_id),
            ),
            Time { time, precision } => {
                self.store_params(connection, (id, property_id, time, precision))
            }
            None => self.store_params(connection, (id, property_id)),
            Unknown => self.store_params(connection, (id, property_id)),
        }
    }
}

impl From<ClaimValueData> for Value {
    fn from(claim_value_data: ClaimValueData) -> Self {
        use ClaimValueData::*;

        match claim_value_data {
            CommonsMedia(string) => Self::String(string),
            GlobeCoordinate {
                lat,
                lon,
                precision,
                globe,
            } => Self::Coordinates {
                latitude: lat,
                longitude: lon,
                precision,
                globe_id: q_id(globe),
            },
            Item(id) => Self::Entity(q_id(id)),
            Property(id) => Self::Entity(p_id(id)),
            String(string) => Self::String(string),
            MonolingualText(text) => Self::String(text.text),
            MultilingualText(texts) => {
                for text in texts {
                    if text.lang.0 == ENGLISH.0 {
                        return Self::String(text.text);
                    }
                }
                Self::None
            }
            ExternalID(string) => Self::String(string),
            Quantity {
                amount,
                lower_bound,
                upper_bound,
                unit,
            } => Self::Quantity {
                amount,
                lower_bound,
                upper_bound,
                unit_id: unit.map(q_id),
            },
            DateTime {
                date_time,
                precision,
            } => Self::Time {
                time: date_time,
                precision,
            },
            Url(string) => Self::String(string),
            MathExpr(string) => Self::String(string),
            GeoShape(string) => Self::String(string),
            MusicNotation(string) => Self::String(string),
            TabularData(string) => Self::String(string),
            Lexeme(id) => Self::Entity(l_id(id)),
            Form(id) => Self::Entity(f_id(id)),
            Sense(id) => Self::Entity(s_id(id)),
            NoValue => Self::None,
            UnknownValue => Self::Unknown,
        }
    }
}
