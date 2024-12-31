# luavisors

A small and scriptable process supervisor for containers using Rust and Lua.

## Why

`luavisors` provides flexible process management through Lua scripting. This
allows you to run and supervise one or more processes within a single container
using custom supervision logic, unlike simple process supervisors such as
[`dumb-init`](https://github.com/Yelp/dumb-init) or [`tini`](https://github.com/krallin/tini).

`luavisors` is a lightweight process supervisor with built-in start, stop, and
scheduling capabilities in a single static binary. Unlike more complex tools
such as `systemd`, `openrc`, `runit`, or `s6`, it doesn't require additional
software to be installed or complicated configuration files.

## Features

- **Lightweight**: `luavisors` is a single static binary with no external
  dependencies.
- **Scriptable**: Supervision logic is written in Lua and enhanced with a bit of
  Rust.
- **Flexible**: Supervise multiple processes with custom start and stop logic.
  Schedule processes to run at specific intervals.
- **Snappy**: Written in Rust with a small asynchronous core to be fast and
  efficient.
- **Portable**: Designed to run in the most [minimal containers](https://github.com/GoogleContainerTools/distroless)
  and environments. No additional software or configuration files are required.
- **Minimal**: Focused solely on process supervision and scheduling with a
  simple API.

## Usage

`luavisors` is intended to run as the main or `init` process (`pid1`) in a
container, but can also be run as a standalone process supervisor. Like other
process supervisors, it will forward signals to child processes and reap zombie
processes.

To run a Lua script with `luavisors`, pass either the script path or the script
itself as a string followed by any additional arguments:

```sh
luavisors [script [args...]]
```

`luavisors` embeds LuaJIT and enables the [Lua 5.2 extensions](https://luajit.org/extensions.html#lua52)
and [FFI library](https://luajit.org/ext_ffi.html), so newer language features
are available and C functions and libraries can be called directly from Lua.

## API

`luavisors` exposes a Lua module called `init`. This module provides the main
functions to start, stop, and schedule processes:

```lua
-- Require the init module
local init = require('init')

-- Get the process id of the parent process
init.pid()

-- Send a signal to a process
init.kill(pid, signal)

-- Sleep for a number of seconds
init.sleep(seconds)

-- Run a function every number of seconds asynchronously
init.every(seconds, function, ...)

-- Execute a child process asynchronously
local child = init.exec(command, ...)

-- Get the child process output
child:stdout()

-- Get the child process errors
child:stderr()

-- Get the child process status
child:status()

-- Kill the child process directly
child:kill()

-- Standard signals are available in the `signal` table
init.signal.SIGTERM
init.signal.SIGKILL
-- etc.
```

## Examples

See the `lua/` directory for more detailed examples:

- [`api.lua`](lua/api.lua): A complete demonstration of the `init` API
- [`simple.lua`](lua/simple.lua): Launches two child processes asynchronously
  and waits for them to finish
- [`advanced.lua`](lua/advanced.lua): Launches a main process and a scheduled
  update process which stops the main process and restarts it after the update
  completes

To run an example:

```sh
luavisors lua/api.lua
# or from the root directory
cargo run -- lua/api.lua
```

See also the included [`Dockerfile`](Dockerfile) for a Docker-based example.

## Development

### Building

To build the executable:

```sh
cargo build --release
```

### Testing

To run the tests:

```sh
cargo test
```
