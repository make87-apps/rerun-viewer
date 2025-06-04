# Rerun Log Viewer
<img src="https://rerun.io/_app/immutable/assets/hero-illustration.CUteR0Ty.svg" alt="Rerun Visualization" width="100%" />
The **Rerun Log Viewer** is a lightweight visualization server that collects and displays logs and data in real time using the [Rerun](https://rerun.io) web viewer. It is designed to work seamlessly with [Vector](https://vector.dev) and applications using the Rerun SDK.

## Features

- 🔌 **TCP sink support for Vector**: Receives newline-delimited JSON logs sent by Vector.
- 🧠 **Dynamic log parsing**: Extracts and visualizes the `message` or `msg` field from log payloads.
- 🌐 **Web-based viewer**: Runs an embedded Rerun web viewer on port `9876` by default.
- 🛰️ **gRPC endpoint**: Exposes a gRPC server for any application using the Rerun SDK to stream data directly.
- ⚙️ **Make87 integration**: Automatically receives configuration such as remote address and memory limits via `MAKE87_CONFIG`.

## Usage

### Environment Configuration

Make87 will inject the required environment variable:

```env
MAKE87_CONFIG='{
  "config": {
    "server_memory_limit": "2GB"
  }
}'
