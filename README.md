# Queryserver

A generic MongoDB query server written in rust.

## Usage

```
USAGE:
    queryserver [FLAGS] [OPTIONS] --connection <uri> --database <database>

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -c, --connection <uri>       Mongodb connection URI
    -d, --database <database>    Database
    -H, --host <host>            Host address
    -p, --password <password>    Password
    -P, --port <port>            Port
    -u, --username <username>    Username
```

Example request:

```
http://localhost:8080/collection_name?limit=5&query={"locale":"en"}
```

## Build

Get [rust](https://www.rust-lang.org/en-US/install.html) and execute:

```
$ cargo build --release
```
