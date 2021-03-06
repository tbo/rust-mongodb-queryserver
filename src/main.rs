// #![deny(warnings)]

extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;
extern crate clap;
extern crate bson;
extern crate mongodb;

extern crate serde;
extern crate serde_json;
extern crate url;
extern crate regex;

use clap::{Arg, App, ArgMatches};
use futures::future::FutureResult;
use std::process;
use std::sync::Arc;
use mongodb::{Client, ThreadedClient};
use mongodb::db::{ThreadedDatabase, DatabaseInner};
use mongodb::coll::options::FindOptions;
use serde_json::Value;
use regex::Regex;
use hyper::{Get, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};
use std::collections::HashMap;
use std::error::Error;

struct QueryService<'a> {
    db: &'a Arc<DatabaseInner>
}

impl<'a> Service for QueryService<'a> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        if req.method() != &Get {
            return get_failure_response(StatusCode::NotFound);
        }
        let collection_regex = Regex::new(r"^/([[:alpha:]_-]*)/?$").unwrap();
        let collection = match collection_regex.captures(req.path()) {
            Some(collection_match) => collection_match.get(1).map_or("", |m| m.as_str()),
            None => return get_failure_response(StatusCode::BadRequest)
        };
        let params = get_query_params(&req);
        let collection = self.db.collection(collection);
        let mut opts = FindOptions::new();
        opts.limit = get_number_or(params.get("limit"), Some(20));
        opts.skip = get_number_or(params.get("skip"), None);
        match to_bson_document(params.get("sort")) {
            Ok(v) => opts.sort = v,
            Err(_) => return get_failure_response(StatusCode::BadRequest)
        }
        let query = match to_bson_document(params.get("query")) {
            Ok(v) => v,
            Err(_) => return get_failure_response(StatusCode::BadRequest)
        };
        match collection.find(query, Some(opts)) {
            Ok(result) => {
                let documents: Vec<String> = result
                    .map(|item| bson::Bson::Document(item.unwrap()).to_string())
                    .collect();
                let output = format!("[{}]", documents.join(","));
                return futures::future::ok(
                    Response::new()
                    .with_header(ContentLength(output.len() as u64))
                    .with_body(output)
                )
            },
            Err(_) => return get_failure_response(StatusCode::InternalServerError),
        }
    }
}

fn get_query_params(request: &Request) -> HashMap<String, String> {
    match request.query() {
        Some(v) => url::form_urlencoded::parse(v.as_bytes()).into_owned().collect(),
        None => HashMap::new()
    }
}

fn get_failure_response(code: StatusCode) -> FutureResult<Response, hyper::Error> {
    futures::future::ok(Response::new().with_status(code))
}

fn to_bson_document(query_option: Option<&String>) -> Result<Option<bson::ordered::OrderedDocument>, Box<Error>> {
    if let Some(query_string) = query_option {
        let json: Value = serde_json::from_str(query_string)?;
        let bson_value = bson::to_bson(&json)?;
        if let Some(bson_document) = bson_value.as_document() {
            return Ok(Some((*bson_document).clone()))
        }
    }
    Ok(None)
}

fn get_number_or(query_option: Option<&String>, default: Option<i64>) -> Option<i64> {
    if let Some(limit_string) = query_option {
        if let Ok(limit) = limit_string.parse::<i64>() {
            return Some(limit);
        } 
    }
    return default
}

fn get_configuration() -> ArgMatches<'static> {
    App::new("MongoDB Query Server")
        .version("0.1")
        .author("Thomas Bonk <tbo@cybo.biz>")
        .arg(Arg::with_name("host")
             .takes_value(true)
             .short("H")
             .long("host")
             .help("Host address"))
        .arg(Arg::with_name("port")
             .takes_value(true)
             .short("P")
             .long("port")
             .help("Port"))
        .arg(Arg::with_name("username")
             .takes_value(true)
             .short("u")
             .long("username")
             .help("Username"))
        .arg(Arg::with_name("password")
             .takes_value(true)
             .short("p")
             .long("password")
             .requires("username")
             .help("Password"))
        .arg(Arg::with_name("v")
             .short("v")
             .multiple(true)
             .help("Sets the level of verbosity"))
        .arg(Arg::with_name("database")
             .value_name("database")
             .short("d")
             .long("database")
             .help("Database")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("connection")
             .value_name("uri")
             .short("c")
             .long("connection")
             .help("Mongodb connection URI")
             .required(true)
             .takes_value(true))
        .get_matches()
}

fn create_database_connection(config: &ArgMatches) -> Arc<DatabaseInner> {
    let connection = config.value_of("connection").unwrap_or("mongodb://127.0.0.1:27017/");
    let client = match Client::with_uri(connection) {
        Err(_) => {
            println!("Unable to connect to {}", connection);
            process::exit(0x0100)
        },
        Ok(val) => val
    };
    let db = client.db(config.value_of("database").unwrap());
    if let Some(ref username) = config.value_of("username") {
        match db.auth(username, config.value_of("password").unwrap_or("")) {
            Err(_) => {
                println!("Failed to authenticate");
                process::exit(0x0100)
            },
            Ok(_) => ()
        }
    }
    return db;
}

fn run(config: &ArgMatches) {
    let db = create_database_connection(&config);
    let host = config.value_of("host").unwrap_or("127.0.0.1");
    let port = config.value_of("port").unwrap_or("80");
    let address = format!("{}:{}", host, port).parse().unwrap();
    let server = match Http::new().bind(&address, move || Ok(QueryService {db: &db})) {
        Err(_) => {
            println!("Unable to bind to {}", address);
            process::exit(0x0100)
        },
        Ok(val) => val
    };
    println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap()
}

fn main() {
    pretty_env_logger::init().unwrap();
    let matches = get_configuration();
    run(&matches);
}
