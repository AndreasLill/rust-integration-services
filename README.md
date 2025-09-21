# Rust Integration Services

A modern, fast, and lightweight integration library written in Rust, designed for memory safety and stability. It simplifies the development of scalable integrations for receiving and sending data, with built-in support for common protocols.

[![Crates.io](https://img.shields.io/crates/v/rust-integration-services.svg)](https://crates.io/crates/rust-integration-services)
![Rust Version](https://img.shields.io/badge/rustc-1.70+-blue.svg)
[![Docs.rs](https://docs.rs/rust-integration-services/badge.svg)](https://docs.rs/rust-integration-services)  
[![License](https://img.shields.io/crates/l/rust-integration-services.svg)](https://github.com/AndreasLill/rust-integration-services#license)

## Installation

Add rust-integration-services to your project Cargo.toml with all or select features.

**All features**
``` toml
[dependencies]
tokio = { version = "1.47.1", features = ["full"] }
rust-integration-services = { version = "0.3.5", features = ["full"] }
```

**With select features**
``` toml
[dependencies]
tokio = { version = "1.47.1", features = ["full"] }
rust-integration-services = { version = "0.3.5", features = ["file", "schedule", "sftp", "http"] }
```

## Features

### File
[Examples](https://github.com/AndreasLill/rust-integration-services/blob/master/src/file/examples.md)

The file module focus on the local file system and is useful for polling files in a directory, copying, moving or writing to a file.


### Http
[Examples](https://github.com/AndreasLill/rust-integration-services/blob/master/src/http/examples.md)

The http module is built on top of the fast and reliable [`hyper`](https://crates.io/crates/hyper) HTTP library, and routing is handled using [`matchit`](https://crates.io/crates/matchit) url router library.

It supports both **HTTP/1.1** and **HTTP/2** protocols, enabling modern, high-performance HTTP communication with automatic protocol negotiation via ALPN (Application-Layer Protocol Negotiation) with dynamic routing for REST.


### Sftp
[Examples](https://github.com/AndreasLill/rust-integration-services/blob/master/src/sftp/examples.md)

Using SFTP requires `openssl` to be installed on the system.  
Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.


### Schedule
[Examples](https://github.com/AndreasLill/rust-integration-services/blob/master/src/schedule/examples.md)

The schedule module assist in scheduling tasks over time, such as hourly, daily or weekly.  
During downtime or maintenance it calculates the next scheduled time automatically on resume.
