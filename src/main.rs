use std::collections::HashMap;

use futures::executor::block_on;

type AbstractValue = u64;

struct LocalDbHandler {
    db: HashMap<String, Box<AbstractValue>>,
}

impl LocalDbHandler { 
    fn new() -> Self {
        Self { db: HashMap::new() }
    }
    async fn update_elem(&mut self, elem: &String, new_value: AbstractValue) {
        self.db.insert(elem.to_string(), Box::new(new_value));
    }
    async fn get_elem(&self, elem: String) -> Option<AbstractValue> {
        self.db.get(&elem).map(|x| *(*x))
    }
    async fn apply_to_all(&mut self, f: fn(AbstractValue)->AbstractValue) {
        for mut i in &mut self.db {

            i.1 = &mut Box::new(f(**i.1));
        }
    }
}

fn main() {
    let dbh = LocalDbHandler::new();
    let future = increment_db(&dbh);
}

async fn increment_db(dbh: &LocalDbHandler) {}
