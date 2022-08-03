use rusqlite::{Connection, Result};

//use serde::{ Serialize, Deserialize };
//use std::collections::HashMap;

/*#[derive(Serialize, Deserialize)]
struct Actor {
    name: String,
}

#[derive(Deserialize)]
struct Effect {
    name: String,
    rnd: usize,
    count: usize,
}

#[derive(Deserialize)]
struct Hat {
    blob: Vec<usize>,
}

#[derive(Deserialize)]
struct Item {
    blob: Vec<usize>,
    sex: usize,
    base: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Job {
    file: String,
    name: String,
    rnd: usize,
    count: usize,
}

#[derive(Deserialize)]
struct Skill {
    internal_name: String,
    name: String,
    description: String,
}

#[derive(Deserialize)]
struct Weapon {
    blob: Vec<usize>,
    name: String,
}*/

pub struct Database {
    pub connection: Connection,
    /*actors: HashMap<usize, Actor>,
    effects: HashMap<usize, Effect>,
    hats: HashMap<usize, Hat>,
    items: HashMap<usize, Item>,
    jobs: HashMap<usize, Job>,
    skills: HashMap<usize, Skill>,
    weapons: HashMap<usize, Weapon>,*/
}

impl Database {

    pub fn new() -> Self {

        let connection = Connection::open("ro.db").unwrap();

        Self { connection }
    }

    pub fn job_name_from_id(&self, id: usize) -> String {

        let mut statement = self.connection.prepare("SELECT * FROM actors WHERE id = ?;").unwrap();
        let mut result = statement.query_map([id], |row| row.get("name")).unwrap();

        result
            .next()
            .map(|name: Result<String, _>| name.unwrap().to_lowercase())
            .unwrap_or("1_f_maria".to_string())
            //.expect(&format!("failed to find actor with id {} in database", id))
    }

    pub fn itme_name_from_id(&self, id: usize) -> String {

        let mut statement = self.connection.prepare("SELECT * FROM items WHERE id = ?;").unwrap();
        let mut result = statement.query_map([id], |row| row.get("en")).unwrap();

        result
            .next()
            .map(|name: Result<String, _>| name.unwrap())
            .unwrap_or("!Failed to find!".to_string())
    }
}
