use hyper::service::{service_fn};
use hyper::{Request, Response};
use std::collections::HashMap;
use tokio::net::TcpListener;
use hyper_util::rt::TokioIo;
use hyper::server::conn::http1;
use http_body_util::{Full, BodyExt};
use std::net::SocketAddr;

use std::convert::Infallible;
use hyper::body::{Bytes, Buf};

use std::sync::Arc;
use futures::lock::Mutex;
use futures::lock::MutexLockFuture;

use std::io::Read;
//use futures::executor::block_on;

#[derive(Hash, PartialEq, Eq, Clone)]
struct UserKey {
    key: String,
}

struct LocalDbHandler {
    db: HashMap<UserKey, String>,
}

impl LocalDbHandler {
    fn new() -> Self {
        Self { db: HashMap::new() }
    }
    async fn update_elem(&mut self, elem: &String, new_value: String) {
        self.db.insert(UserKey { key: elem.clone() }, new_value);
    }
    //async fn get_elem(&self, elem: String) -> Option<String> {

    //   self.db.get(&elem).map(|x| *(*x))
    // }
    async fn apply_to_all(&mut self, f: fn(String) -> String) {
        //      self.db = self.db.into_iter().map(|(k, v)|(k, f(v))).collect();
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
}



#[tokio::main]
async fn main() {
    let mut dbh = Arc::new(Mutex::new(LocalDbHandler::new()));
    async {
        dbh.clone().lock().await.update_elem(&"bob".to_string(), "dhbfjnbgjh".to_string())
            .await;
        dbh.clone().lock().await.dump_terminal().await;
        dbh.clone().lock().await.apply_to_all(|x| x.to_uppercase()).await;
        dbh.clone().lock().await.dump_terminal().await;
    }
    .await;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Server listening on http://{}", addr);


    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await.unwrap();

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);
        let dbhh = dbh.clone();
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
            .serve_connection(io, service_fn(|req| hello(req, dbhh.clone())))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn hello(r: Request<hyper::body::Incoming>, dbh: Arc<Mutex<LocalDbHandler>>) -> Result<Response<Full<Bytes>>, Infallible> {
    //let mut buf = vec![];
    //let mut test = r.into_body().collect().await.unwrap().aggregate().reader();
    let test = r.into_body().collect().await.unwrap().to_bytes();
    let test = std::str::from_utf8(&test);
    match test {
        Ok(text) => {
            println!("body: {}", text);
           // let re = Regex::new(r"(a-z)+;(a-zA-Z0-9)+$").unwrap();
           // if (text.contains("GET"))
         //   }
    },
        Err(_) => println!("Failed to get body"),
    }


    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}
