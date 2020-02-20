# search-rs
Small project to use PostgreSQL to make a geometry search index. Made with Rust, can communicate with it via grcp.
It does not have authentication because it is ment to be internal.

## Why?
First project in Rust. But I wanted to do something that made actual sense.

## What does it use?
 - [tonic](https://github.com/hyperium/tonic)
 - [tokio-postgres (from rust-postgres)](https://github.com/sfackler/rust-postgres)
