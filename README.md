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
tokio = { version = "1.49.0", features = ["full"] }
rust-integration-services = { version = "0.5.3", features = ["full"] }
```

**With select features**
``` toml
[dependencies]
tokio = { version = "1.49.0", features = ["full"] }
rust-integration-services = { version = "0.5.3", features = ["file", "scheduler", "sftp", "http", "smtp", "s3"] }
```

## Features

### File

The file module focus on the local file system and is useful for polling files in a directory, copying, moving or writing to a file.


### Http

The http module is built on top of the fast and reliable [`hyper`](https://crates.io/crates/hyper) crate, and routing is handled using [`matchit`](https://crates.io/crates/matchit) crate.

It supports both **HTTP/1.1** and **HTTP/2** protocols, enabling modern, high-performance HTTP communication with automatic protocol negotiation via ALPN (Application-Layer Protocol Negotiation) with dynamic routing for REST.

### S3

The S3 module is built on top of the [`AWS SDK`](https://crates.io/crates/aws-sdk-s3) and provides a simplified, easy-to-use client for interacting with Amazon S3 and other generic S3 services like minio. It abstracts common operations into a clean, versatile interface while retaining the flexibility of the underlying SDK.

### Sftp

The sftp module is built on top of the low-level [`russh`](https://crates.io/crates/russh) and [`russh-sftp`](https://crates.io/crates/russh-sftp) crates.  

These crates provide direct access to the SSH transport layer and the SFTP protocol, giving full control over connection management, authentication, and file transfer operations.  

### Smtp

The SMTP module is built on top of the reliable [`lettre`](https://crates.io/crates/lettre) crate.

`lettre` provides an asynchronous, rust-native SMTP support for secure connections via TLS.

### Scheduler

The scheduler module assist in scheduling tasks over time, such as hourly, daily or weekly.  
During downtime or maintenance it calculates the next scheduled time automatically on resume.
