# inoli

##### Project Status:
Architecture is sound, **TUI** is wack, it *panics!* a lot, but we are getting there.

<img width="270" src="https://user-images.githubusercontent.com/2061234/194773731-b697247e-193c-4f61-b55a-2a919178f7bc.png" alt="TUI">


## Architecture
**inoli** is a tiny server, that interacts with your wristband, and uses Unix socket to transmit messages and receive commands from clients.

## Building

##### Cloning repository
```bash
git clone https://github.com/Elvyria/inoli
cd inoli
```

##### Server
```bash
cargo build --release
```

##### TUI Client
```bash
cd tui
go build -ldflags "-s -w".
```

## Communication Protocol

#### Message:
Server emits messages when device reports something, or it received a command.  

| Name   | Type    | Size | Value |
|--------|---------|------|-------|
| Magic  | char[3] | 3    | MSG   |
| Type   | uint8   | 1    |       |
| Data   |         | n    |       |

#### Command
Clients can send commands to server.

| Name      | Type    | Size | Value | Notes            |
|-----------|---------|------|-------|------------------|
| Magic     | char[3] | 3    | MSG   |                  |
| Type      | uint8   | 1    |       |                  |
| Operation | uint8   | 1    | 0, 1  | Get, Set         |
| Data      |         | n    |       |                  |

## Adding Device

## Supported Devices
* MiBand 1(S)
