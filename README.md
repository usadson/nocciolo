# Nocciolo
A simple kernel written for fun and profit.

## Building & Running
Since the project uses [Cargo](https://doc.rust-lang.org/cargo/), there is no need to explicitly build the kernel,
since this is implicitly done by the `cargo run` command.

To run the operating system, the [`os`](./os/) package builds the kernel and includes the prerequisites for booting, and
invokes the amazing [QEMU](https://www.qemu.org/) emulator. The `os` package has a couple of subcommands for running
the kernel, notably to specify if you want to run under **UEFI** or **BIOS**.

### UEFI mode (recommended)
To run the kernel under [UEFI](https://uefi.org/), you can use the `uefi` subcommand.
```shell
cargo run uefi
```

And for using BIOS:

### BIOS mode
```shell
cargo run bios
```

## Debugging
To use [GDB](https://sourceware.org/gdb/) or [LLDB](https://lldb.llvm.org/) with the kernel, you can use the `debug`
option with the `uefi` command:
```shell
cargo run uefi debug
```

This will effectively request QEMU to act like a GDB Server and wait for the debugger. You can attach a remote debugger
at `localhost:1234`, or you can use the `gdb`/`lldb` subcommands. The latter is preferred, since that way we can give
the debugger the correct symbols, a couple of handy breakpoints and the object address slide.
```shell
cargo run gdb
# or
cargo run lldb
```
> **NOTE:** I currently use macOS for debugging, and using LLVM helps to parse x86-64 ELF objects (for symbols) while
> running in an obvious AArch64 Mach-O environment. GDB might not work perfectly yet. 

## Quick Links
* [ACPI 6.5 Specification](https://uefi.org/specs/ACPI/6.5)
* [OSDev Wiki](https://wiki.osdev.org/)
