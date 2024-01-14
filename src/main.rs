use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use hyper::body::Bytes;
use std::convert::Infallible;

use futures::lock::Mutex;
use std::sync::Arc;

use tokio::time::Duration;
use arl::RateLimiter;

use regex::Regex;

#[derive(Hash, PartialEq, Eq, Clone)]
struct UserKey {
    key: String,
}

impl UserKey {
    fn new(key: String) -> Self {
        Self { key: key }
    }
}

struct LocalDbHandler {
    db: HashMap<UserKey, String>,
}

enum SetResult {
    SucessNewEntry,
    SucessEntryExisted,
    // Failure,
}

impl LocalDbHandler {
    fn new() -> Self {
        Self { db: HashMap::new() }
    }
    async fn update_elem(&mut self, elem: &String, new_value: String) {
        self.db.insert(UserKey { key: elem.clone() }, new_value);
    }

    async fn apply_to_all(&mut self, f: fn(String) -> String) {
        self.db = self
            .db
            .iter_mut()
            .map(|(k, v)| (k.clone(), f(v.to_string())))
            .collect();
    }
    async fn dump_terminal(&self) {
        println!("All mappings:");
        for (k, v) in self.db.iter() {
            println!("Key: {}, value: {}", k.key, v);
        }
    }
    async fn get(&self, user: &str) -> Option<String> {
        self.db.get(&UserKey::new(user.to_string())).cloned()
    }

    async fn set(&mut self, user: &str, value: &str) -> SetResult {
        match self
            .db
            .insert(UserKey::new(user.to_string()), value.to_string())
        {
            Some(_) => SetResult::SucessEntryExisted,
            None => SetResult::SucessNewEntry,
        }
    }
}

const RL_TIMESLOT_SIZE_SECONDS: u64 = 10;
const RL_MAX_QUERIES_PER_TIMESLOT: usize = 5;

#[tokio::main]
async fn main() {
    let dbh = Arc::new(Mutex::new(LocalDbHandler::new()));
    // Async stuff experiments
    async {
        dbh.clone()
            .lock()
            .await
            .update_elem(&"bob".to_string(), "dhbfjnbgjh".to_string())
            .await;
        dbh.clone().lock().await.dump_terminal().await;
        dbh.clone()
            .lock()
            .await
            .apply_to_all(|x| x.to_uppercase())
            .await;
        dbh.clone().lock().await.dump_terminal().await;
    }
    .await;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Server listening on http://{}", addr);
    let limiter = RateLimiter::new(RL_MAX_QUERIES_PER_TIMESLOT, Duration::from_secs(RL_TIMESLOT_SIZE_SECONDS));

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await.unwrap();

        let io = TokioIo::new(stream);
        let dbhh = dbh.clone();
        let limiter = limiter.clone();
        
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            limiter.wait().await;
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(|req| process_users(req, dbhh.clone())))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn process_users(
    r: Request<hyper::body::Incoming>,
    dbh: Arc<Mutex<LocalDbHandler>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let test = r.into_body().collect().await.unwrap().to_bytes();
    let test = std::str::from_utf8(&test);
    match test {
        Ok(text) => {
            println!("body: {}", text);
            let re = Regex::new(r"([a-z]+);+([a-zA-Z0-9]+);*(.*)$").unwrap();
            let groups = re.captures(text);
            if let Some(groups) = groups {
                let command = groups.get(1);
                let user = groups.get(2);
                if let Some(command) = command {
                    if let Some(user) = user {
                        match command.as_str() {
                            "get" => {
                                let res = dbh.lock().await.get(user.as_str()).await;
                                match res {
                                    Some(s) => {
                                        let returned_text = format!("Success, value is {}\n", s);
                                        return Ok(Response::new(Full::new(Bytes::from(
                                            returned_text,
                                        ))));
                                    }
                                    None => {
                                        return Ok(Response::new(Full::new(Bytes::from(
                                            "No user could be found\n",
                                        ))))
                                    }
                                }
                            }
                            "set" => {
                                if let Some(value) = groups.get(3) {
                                    let res =
                                        dbh.lock().await.set(user.as_str(), value.as_str()).await;
                                    match res {
                                        SetResult::SucessNewEntry => return Ok(Response::new(Full::new(Bytes::from(
                                            "Entry added\n",
                                        )))),
                                        SetResult::SucessEntryExisted => return Ok(Response::new(Full::new(Bytes::from(
                                            "Entry added, user already existed, value was replaced\n",
                                        )))),
                                    }
                                } else {
                                    return Ok(Response::new(Full::new(Bytes::from(
                                        "Invalid set value\n",
                                    ))));
                                }
                            }
                            _ => {
                                return Ok(Response::new(Full::new(Bytes::from(
                                    "Invalid command\n",
                                ))));
                            }
                        }
                    } else {
                        return Ok(Response::new(Full::new(Bytes::from("Invalid user\n"))));
                    }
                } else {
                    return Ok(Response::new(Full::new(Bytes::from("Invalid command\n"))));
                }
            }
        }
        Err(_) => println!("Failed to get body"),
    }

    Ok(Response::new(Full::new(Bytes::from("Could not process request\n"))))
}
