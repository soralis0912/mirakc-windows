# Running mirakc on Windows

This fork makes it possible to run mirakc natively on Windows with tuners accessed through
BonDriver DLLs.

* `mirakc.exe` builds and runs on Windows (x86_64)
* `mirakc.exe service install` registers mirakc as a Windows service
* Tuners are accessed through [BonTsCat], a small command in the
  [EDCB fork][edcb-fork] that opens a BonDriver and writes TS packets to STDOUT
  (like `recpt1` on Linux)
* [mirakc-arib] is built for Windows in the [mirakc-arib fork][mirakc-arib-fork]

## Limitations

* UNIX domain sockets (`server.addrs[].unix`) are not supported and ignored with a warning
* `mirakc-timeshift-fs` (FUSE) is not available
* Tuner processes are terminated with `TerminateProcess()` when streaming stops, in the same way
  as `SIGKILL` on Linux.  BonDrivers that cannot recover from abnormal termination may need to
  be checked

## Build

On Windows (MSVC or GNU toolchain):

```shell
cargo build --release -p mirakc
```

Cross-compiling from Linux/WSL with [cargo-zigbuild]:

```shell
rustup target add x86_64-pc-windows-gnu
cargo zigbuild --release --target x86_64-pc-windows-gnu -p mirakc
```

## Installation

Put the files in a folder like below:

```
C:\mirakc\
  mirakc.exe
  strings.yml      # copy from resources/strings.yml
  config.yml
  mirakc-arib.exe  # from the mirakc-arib fork
  BonTsCat.exe     # from the EDCB fork
  BonDriver\
    BonDriver_XXX.dll
    BonDriver_XXX.ini (if needed)
```

`resource.strings-yaml` defaults to `strings.yml` in the folder containing `mirakc.exe`.

The folder containing `mirakc.exe` is added to `PATH` at startup.  So, commands placed in the
folder such as `mirakc-arib.exe` can be used without changing the system `PATH`.  This is
convenient especially for the Windows service.

## Find channels of a BonDriver

```shell
C:\mirakc> BonTsCat.exe -d BonDriver_XXX.dll -l
0-0     UHF     13ch
0-1     UHF     14ch
...
1-0     BS      BS01/TS0
```

The first column is `space-ch` and can be specified to `-c` of `BonTsCat.exe` as is.

## config.yml

Use `space-ch` as `channels[].channel` and pass it to `BonTsCat.exe` in `tuners[].command`.
Use `-` as the separator because the channel is also used in URL paths of the Web API.

```yaml
epg:
  cache-dir: C:\mirakc\epg

server:
  addrs:
    - http: '0.0.0.0:40772'

channels:
  - name: NHK
    type: GR
    channel: '0-14'
  - name: BS1
    type: BS
    channel: '1-0'

tuners:
  - name: Tuner0
    types: [GR, BS]
    command: >-
      C:\mirakc\BonTsCat.exe -d BonDriver_XXX.dll -c {{{channel}}}
```

On Windows, only the program name is parsed by mirakc and the remaining part of a command line is
passed to the program as is.  So, backslashes in paths don't need to be escaped.  Quote the
program name with `"` if it contains spaces.

## Run in a console

```shell
C:\mirakc> mirakc.exe -c C:\mirakc\config.yml
```

Press Ctrl+C to stop.

## Run as a Windows service

Run the following commands in an elevated command prompt:

```shell
C:\mirakc> mirakc.exe -c C:\mirakc\config.yml service install
C:\mirakc> sc start mirakc
```

Options of `service install`:

* `--name <NAME>`: service name (default: `mirakc`)
* `--display-name <NAME>`: display name (default: `mirakc`)
* `--start auto|manual|disabled`: start type (default: `auto`)
* `--log-filter <FILTER>`: value of `RUST_LOG` in the service (default: `info`)

Global options such as `--log-format` and `--log-file` specified at the installation are used in
the service.  Logs are written to `mirakc.log` in the folder containing `config.yml` unless
`--log-file` is specified.

The service runs as `LocalSystem`.

To uninstall:

```shell
C:\mirakc> mirakc.exe -c C:\mirakc\config.yml service uninstall
```

## Roadmap

* Optional B-CAS decoding in BonTsCat (B25Decoder.dll)

[BonTsCat]: https://github.com/soralis0912/EDCB/tree/mirakc-bondriver/Tools/BonTsCat
[edcb-fork]: https://github.com/soralis0912/EDCB
[mirakc-arib]: https://github.com/mirakc/mirakc-arib
[mirakc-arib-fork]: https://github.com/soralis0912/mirakc-arib/tree/windows
[cargo-zigbuild]: https://github.com/rust-cross/cargo-zigbuild
