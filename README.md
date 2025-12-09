# nu_plugin_socket

A Nushell plugin for low-level TCP and UDP socket communication.

This plugin provides commands to create network clients and servers directly within Nushell, designed to integrate seamlessly with its pipelines and structured data. It serves as a Nu-idiomatic alternative to traditional tools like `netcat`, allowing for powerful, self-contained network scripting.

## Features

*   **TCP and UDP Client:** Create network clients for both protocols using the `socket connect` command.
*   **Streaming TCP Client:** The TCP client is a true stream, outputting data as it arrives from the server.
*   **Concurrent TCP Server:** Create multi-threaded servers with the `socket listen` command, handling each connection in a separate thread.
*   **Nushell-Native Server Logic:** Define server behavior using Nushell closures, allowing you to process requests and generate replies with the full power of the shell.
*   **Service Name Resolution:** Supports standard service names (e.g., `http`, `whois`) in place of port numbers.
*   **Configurable Timeouts:** All network operations use timeouts to prevent hangs, with defaults that can be overridden by a flag or global Nushell configuration.

## Installation

### Prerequisites

You must have a Rust toolchain and the Nushell shell installed.

### 1. Install the Plugin Binary

Install the plugin from crates.io (once published) or directly from the source code using `cargo`:

```sh
cargo install nu_plugin_socket
```
This will build the plugin and place the `nu_plugin_socket` binary in your cargo home directory (typically `~/.cargo/bin/`).

### 2. Register the Plugin with Nushell

To use the plugin, you must register it with Nushell. You can do this temporarily by running:

```nushell
> plugin add ~/.cargo/bin/nu_plugin_socket
```

To make the plugin available in every shell session, add this command to your Nushell configuration file (which you can open by running `config nu`):

```nushell
# in config.nu
> plugin add ~/.cargo/bin/nu_plugin_socket
```

After restarting Nushell, the `socket` command and its subcommands will be available.

## Usage and Examples

### `socket connect` (Client)

The `connect` command is a filter that reads data from its standard input, sends it to the server, and streams the server's reply to its standard output.

**Example 1: Simple HTTP GET Request (TCP)**

This command sends a raw HTTP GET request and decodes the server's binary response into a string.

```nushell
> "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n" | socket connect example.com http | decode
```

**Example 2: WHOIS Query (TCP)**

This command queries a WHOIS server for information about the `.il` domain.

```nushell
> "il\r\n" | socket connect whois.iana.org 43| decode
```

**Example 3: DNS Query (UDP)**

This command sends a minimal, valid DNS request packet to a Google server using the `--udp` flag and receives a 44-byte binary response.

```nushell
> 0x[12340100000100000000000006676f6f676c6503636f6d0000010001] | socket connect 8.8.8.8 53 --udp
```

### `socket listen` (Server)

The `listen` command starts a server that executes a Nushell closure for each incoming connection. The closure receives the client's request as a binary argument, and its return value (which must be a string or binary) is sent back as the reply.

**Example: A Simple Echo Server**

1.  **Start the server in one terminal:** This command starts a server that echoes back any data it receives. It will run until you press `Ctrl-C`.

    ```nushell
    > socket listen 0.0.0.0 8080 { |request| $request }
    Listening on 0.0.0.0:8080... (Press Ctrl+C to stop)
    ```

2.  **Connect with a client in a second terminal:**

    ```nushell
    > "hello world" | socket connect 127.0.0.1 8080 | decode
    hello world
    ```

**Example: A One-Shot Server**

The `--single` flag causes the server to terminate after handling its first connection, which is useful for scripting.

```nushell
> socket listen 127.0.0.1 8081 --single { |req| $"you sent: ($req | decode)" }
```

## Commands Reference

### `socket connect <host> <port>`

*   `host`: The hostname or IP address to connect to.
*   `port`: The port number or standard service name (e.g., `80` or `http`).
*   `--timeout <duration>`: Sets a timeout for network operations (e.g., `5sec`, `500ms`). Overrides any configured default.
*   `--udp`: Use the UDP protocol instead of the default TCP.

### `socket listen <host> <port> <closure>`

*   `host`: The hostname or IP address to listen on (e.g., `127.0.0.1` for local, `0.0.0.0` for all interfaces).
*   `port`: The port number to bind to.
*   `closure`: A Nushell closure that takes one argument (the binary request from the client) and returns a string or binary value to be sent as the reply.
*   `--single`: Terminate the server after handling the first connection.

## Configuration

You can set a default timeout for all `socket` commands by adding a setting to your Nushell configuration (`config nu`). The command-line `--timeout` flag will always take precedence.

```nushell
# in config.nu

# Set a default timeout of 5 seconds for all `socket` plugin commands.
let-env config = ($env.config | upsert plugins {
    socket: {
        timeout: 5sec
    }
})
```

## Building from Source

1.  Clone the repository:
    ```sh
    git clone https://github.com/punund/nu_plugin_socket.git
    cd nu_plugin_socket
    ```
2.  Build the plugin:
    ```sh
    cargo build --release
    ```
3.  The binary will be located at `target/release/nu_plugin_socket`. You can then register this binary with Nushell as described in the installation section.

## License

This project is licensed under the MIT License.
