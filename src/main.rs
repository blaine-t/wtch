use std::fs;
use std::fs::DirEntry;
use std::net::SocketAddr;
use std::process::Command;

use http_body_util::Empty;
use http_body_util::Full;
use http_body_util::{combinators::BoxBody, BodyExt};

use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, StatusCode};
use hyper::{Request, Response};

use hyper_util::rt::TokioIo;

use tokio::net::TcpListener;

fn run_command(path: &DirEntry) -> String {
    let mut command = Command::new(
        fs::read_dir(path.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .to_owned(),
    );
    let output = command.output().unwrap();

    String::from_utf8(output.stdout).unwrap()
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if req.method() == Method::GET {
        let uri_path = req.uri().path()[1..].to_string();
        let paths = fs::read_dir("./endpoints").unwrap();
        for path in paths {
            let path = path.unwrap();
            if uri_path == path.file_name().to_str().unwrap() {
                return Ok(Response::new(full(run_command(&path))));
            }
        }
    }
    let mut not_found = Response::new(empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

// We create some utility functions to make Empty and Full bodies
// fit our broadened Response body type.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize the endpoints directory if it hasn't already
    fs::create_dir_all("./endpoints").unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `echo` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
