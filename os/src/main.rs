// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::process::Command;
use std::io::{ErrorKind, Write};

fn main() -> Result<(), std::io::Error> {
    let mut cmd;

    setup_env();

    let s = std::env::args().skip(1).next();
    match s.as_ref().map(|x| x.as_str()) {
        Some("lldb") => {
            cmd = create_lldb_command()?;
        }

        Some("gdb") => {
            cmd = create_gdb_command()?;
        }

        Some("bios") => {
            cmd = create_qemu_cmd();
            let bios_path = env!("BIOS_PATH");

            cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));
        }

        Some("uefi") => {
            cmd = create_qemu_cmd();
            let uefi_path = env!("UEFI_PATH");

            cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
            cmd.arg("-drive").arg(format!("format=raw,file={uefi_path}"));
        }

        Some(command) => {
            println!("OS> Unknown command `{command}`");
            return Ok(());
        }

        None => {
            print!("OS> No command supplied! `uefi`, `bios`, `lldb`");
            return Ok(());
        }
    }


    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}

fn create_qemu_cmd() -> Command {
    let mut cmd = Command::new("qemu-system-x86_64");

    // Prevent rebooting because of faults
    cmd.arg("-no-reboot");
    cmd.arg("-no-shutdown");

    // Get CPU reset info
    cmd.args(["-d", "int"]);

    // GDB stuff
    cmd.args(["-s", "-S"]);

    // Attach serial output to stdio
    cmd.args(["-serial", "stdio"]);

    cmd
}

fn create_lldb_command() -> Result<Command, std::io::Error> {
    let mut cmd = Command::new("lldb");

    let path = "target/lldb-input";
    println!("Current dir: {:?}", std::env::current_dir());

    let mut file = std::fs::File::create(path)?;
    writeln!(file, "target create {}", env!("KERNEL"))?;
    writeln!(file, "target modules load --file {} --slide 0x8000000000", env!("KERNEL"))?;
    writeln!(file, "gdb-remote localhost:1234")?;
    writeln!(file, "breakpoint set --name kernel_main")?;
    writeln!(file, "breakpoint set --name page_fault_handler")?;
    writeln!(file, "breakpoint set --name double_fault_handler")?;
    writeln!(file, "breakpoint set --name breakpoint_handler")?;
    writeln!(file, "breakpoint set --name hlt_loop")?;
    writeln!(file, "continue")?;

    cmd.args(["-s", path]);

    Ok(cmd)
}

fn create_gdb_command() -> Result<Command, std::io::Error> {
    let mut name = "gdb";

    #[cfg(target_os = "macos")]
    if which::which("ggdb").is_ok() {
        name = "ggdb";
    }

    let mut cmd = Command::new(name);

    let path = "target/.gdbinit";

    let mut file = std::fs::File::create(path)?;
    writeln!(file, "file {}", env!("KERNEL"))?;
    writeln!(file, "target remote localhost:1234")?;
    writeln!(file, "b kernel_main")?;
    writeln!(file, "continue")?;

    cmd.current_dir("target");

    Ok(cmd)
}


#[cfg(unix)]
fn setup_env() {
    if !std::path::Path::new(env!("KERNEL")).exists() {
        panic!("RIP file: {}", env!("KERNEL"));
    }
    // let Err(e) =
    let _ =
    std::os::unix::fs::symlink(env!("KERNEL"), "target/kernel-bin")
    // else { return }
    ;
    // if e.kind() == ErrorKind::AlreadyExists {
    //     return;
    // }

    // println!("File: {}", env!("KERNEL"));
    // Err::<(), std::io::Error>(e).unwrap();
}
