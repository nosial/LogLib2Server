# LogLib2 Server

A lightweight logging event server written in Rust for [LogLib2](https://github.com/nosial/LogLib2). Receives structured
log events via TCP or UDP, formats them for console output with color-coded levels, and optionally persists them to daily
rotating JSONL files.

## Table of contents

<!-- TOC -->
* [LogLib2 Server](#loglib2-server)
  * [Table of contents](#table-of-contents)
  * [Features](#features)
  * [Architecture](#architecture)
  * [Usage](#usage)
    * [Core Options](#core-options)
    * [Console Display Options](#console-display-options)
    * [Help and Version](#help-and-version)
  * [Log Event Format](#log-event-format)
    * [Example Log Event](#example-log-event)
    * [Level Mapping](#level-mapping)
  * [Console Output](#console-output)
  * [File Logging](#file-logging)
  * [Makefile Commands](#makefile-commands)
  * [Building from Source](#building-from-source)
    * [Prerequisites](#prerequisites)
    * [Build](#build)
    * [Installation](#installation)
  * [License](#license)
<!-- TOC -->

## Features

- **Dual protocol support** — receive log events over TCP or UDP, or both simultaneously
- **Colorized console output** — level-aware color coding (DBG, VRB, INFO, WRN, ERR, CRT)
- **Structured log events** — application name, timestamp, level, message, stack traces, and exception details
- **Daily rotating file logging** — optionally write JSONL files that rotate daily (`log2026-05-28.jsonl`)
- **Stack trace formatting** — three modes: none, basic, or full detail
- **Graceful shutdown** — drains events on SIGINT/SIGTERM
- **No OpenSSL dependency** — pure Rust TLS is not needed; the server is lightweight with minimal dependencies

---

## Architecture

```
Client (TCP/UDP)  -->  LogLib2 Server  -->  Console (stdout/stderr)
                                       -->  File Logger (JSONL)
```

1. A client sends a JSON-encoded `LogEvent` over TCP or UDP to the server
2. The server parses the event
3. The event is forwarded to the event processor via a channel
4. The event processor formats the event for console output (with optional color coding) and writes it to stdout/stderr
5. If file logging is enabled, the event is also appended to the daily rotating JSONL file

---

## Usage

```bash
LogLib2Server -p 5131 -o /var/log/myapp
```

### Core Options

All options can be set via CLI flags.

| Option              | Short | Default   | Description                                                    |
|---------------------|-------|-----------|----------------------------------------------------------------|
| `--host`            | `-H`  | `0.0.0.0` | Bind address for the server                                    |
| `--port`            | `-p`  | `5131`    | Port to listen on                                              |
| `--protocol`        |       | `both`    | Protocol to use: `tcp`, `udp`, or `both`                       |
| `--output-path`     | `-o`  | (none)    | Directory path for daily rotating JSONL log files              |

### Console Display Options

| Option           | Short | Default | Description                                                    |
|------------------|-------|---------|----------------------------------------------------------------|
| `--trace`        |       | `basic` | Stack trace format: `none`, `basic`, or `full`                 |
| `--show-st`      |       | `false` | Always show stack traces in the trace section of the event     |
| `--no-color`     |       | `false` | Disable ANSI color codes in console output                     |

### Help and Version

| Option      | Short | Description               |
|-------------|-------|---------------------------|
| `--help`    | `-h`  | Print help information    |
| `--version` | `-V`  | Print version information |

---

## Log Event Format

Log events are JSON-encoded objects sent as a single line over TCP or as a UDP datagram.

| Field              | Type     | Description                                            |
|--------------------|----------|--------------------------------------------------------|
| `application_name` | `string` | Name of the application sending the event              |
| `timestamp`        | `any`    | Optional timestamp value                               |
| `level`            | `string` | Log level (see [Level Mapping](#level-mapping))        |
| `message`          | `string` | Log message text                                       |
| `trace`            | `string` | Optional free-form trace string                        |
| `stack_trace`      | `array`  | Structured stack trace frames (canonical name)         |
| `traces`           | `array`  | Deprecated alias for `stack_trace` (accepted on input) |
| `exception`        | `object` | Optional exception details                             |
| `source`           | `string` | Set by the server (client address); ignored on input   |

### Example Log Event

```json
{
  "application_name": "com.example.myapp",
  "timestamp": "2026-05-28T12:00:00Z",
  "level": "ERROR",
  "message": "Something went wrong",
  "exception": {
    "name": "RuntimeException",
    "message": "Null reference",
    "code": 500,
    "file": "/app/src/main.php",
    "line": 42,
    "trace": [
      { "file": "/app/src/main.php", "line": 42, "function": "process", "class": "MyApp\\Handler" }
    ],
    "previous": {
      "name": "Error",
      "message": "Undefined variable \\$foo",
      "file": "/app/src/lib.php",
      "line": 10
    }
  },
  "stack_trace": [
    { "file": "/app/src/main.php", "line": 42, "function": "process", "class": "MyApp\\Handler", "call_type": "::" }
  ]
}
```

### Level Mapping

The server normalizes log levels to short codes:

| Input                     | Normalized | Console Color |
|---------------------------|------------|---------------|
| `DBG`, `DEBUG`            | `DBG`      | Cyan          |
| `VRB`, `VERBOSE`          | `VRB`      | Blue          |
| `INFO`                    | `INFO`     | Green         |
| `WRN`, `WARNING`          | `WRN`      | Yellow        |
| `ERR`, `ERROR`            | `ERR`      | Red           |
| `CRT`, `CRITICAL`         | `CRT`      | Red on White  |
| (unknown)                 | `INFO`     | Green         |

---

## Console Output

Events at level `WRN`, `ERR`, and `CRT` are written to stderr; all others go to stdout. When color is enabled, each
event is prefixed with the appropriate ANSI color code and the application name is rendered in bold.

Example output:

```
[MyApp] Something went wrong
RuntimeException (500): Null reference
  File: /app/src/main.php:42
  Stack Trace:
    - MyApp\Handler::process
  Caused by: Error: Undefined variable $foo
    File: /app/src/lib.php:10
```

If `--trace full` is set, stack trace frames include file paths and line numbers in parentheses:

```
    - MyApp\Handler::process (/app/src/main.php:42)
```

---

## File Logging

When `--output-path` is specified, the server writes all received events to daily rotating JSONL files in the given
directory. Each line is a complete JSON representation of the `LogEvent`.

```
/var/log/myapp/
  log2026-05-28.jsonl
  log2026-05-29.jsonl
  log2026-05-30.jsonl
```

Files are created with `.jsonl` extension and rotated at midnight based on the server's local time.

---

## Makefile Commands

| Command                                | Description                                 |
|----------------------------------------|---------------------------------------------|
| `make` / `make build` / `make release` | Build in release mode                       |
| `make debug`                           | Build in debug mode                         |
| `make check`                           | Run clippy with warnings denied             |
| `make fmt`                             | Format code with `cargo fmt`                |
| `make fmt-check`                       | Check formatting without changes            |
| `make clean`                           | Clean build artifacts                       |
| `make install`                         | Install binary via `cargo install --path .` |

---

## Building from Source

### Prerequisites

- Rust 1.75 or later (edition 2021)
- Cargo

### Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/LogLib2Server`.

### Installation

```bash
cargo install --path .
```

Or copy the binary directly:

```bash
cp target/release/LogLib2Server /usr/local/bin/
```

---

## License

The project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
