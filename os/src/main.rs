// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::process::Command;
use std::io::Write;

fn main() -> Result<(), std::io::Error> {
    let mut cmd;

    setup_env()?;

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

        Some("info") => {
            println!("OS> UEFI_PATH: {}", env!("UEFI_PATH"));
            println!("OS> BIOS_PATH: {}", env!("BIOS_PATH"));
            println!("OS> KERNEL: {}", env!("KERNEL"));
            return Ok(());
        }

        Some("iso") => {
            if !does_command_exist("mkisofs") {
                println!("OS> CLI tool `mkisofs` not found\n");
                println!("URL: https://codeberg.org/schilytools/schilytools\n");
                println!("Download Instructions:");
                println!("| OS      | Package Manager | Instructions");
                println!("|---------|-----------------|----------------------------");
                println!("| macOS   | macOS           | brew install cdrtools");
                println!("| macOS   | MacPorts        | sudo port install cdrtools");
                println!("| Linux   | apt             | sudo apt install cdrkit");
                println!("| Windows | Chocolately     | N/A");
                println!("| Windows | Scoop           | scoop install main/cdrtools");
                println!("| Windows | winget          | N/A");
                return Ok(());
            }

            let dir = "target/iso";
            let img = "nocciolo.img";

            _ = std::fs::create_dir(dir);
            std::fs::copy(env!("BIOS_PATH"), &format!("{dir}/{img}"))?;

            cmd = Command::new("mkisofs");
            cmd.arg("-U"); // Allows "Untranslated" filenames
            cmd.arg("-no-emul-boot");
            cmd.args(["-b", img]);
            cmd.args(["-hide", img]);
            cmd.args(["-V", "Nocciolo"]);
            cmd.args(["-iso-level", "3"]);
            cmd.args(["-o", &format!("{dir}/nocciolo.iso")]);
            cmd.args([dir]);
        }

        Some("bochs") => {
            let dir = format!("{}/../tools/", env!("CARGO_MANIFEST_DIR"));
            std::env::set_current_dir(dir)?;

            cmd = create_bochs_cmd();
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

fn create_bochs_cmd() -> Command {
    let mut cmd = Command::new("bochs");

    cmd.arg("-q");

    cmd
}

fn create_qemu_cmd() -> Command {
    let mut cmd = Command::new("qemu-system-x86_64");

    // Prevent rebooting because of faults
    cmd.arg("-no-reboot");

    // Get CPU reset info
    cmd.args(["-d", "int"]);

    // GDB stuff
    if std::env::args().nth(2) == Some("debug".into()) {
        cmd.args(["-s", "-S"]);
    }

    if std::env::args().nth(2) == Some("monitor".into()) {
        cmd.args(["-monitor", "stdio"]);
    } else {
        // Attach serial output to stdio
        cmd.args(["-serial", "stdio"]);
    }

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
    writeln!(file, "breakpoint set --name alloc_error_handler")?;
    writeln!(file, "breakpoint set --name page_fault_handler")?;
    writeln!(file, "breakpoint set --name double_fault_handler")?;
    writeln!(file, "breakpoint set --name breakpoint_handler")?;
    writeln!(file, "breakpoint set --name hlt_loop")?;
    writeln!(file, "breakpoint set --name fallback_allocator_oom")?;
    writeln!(file, "breakpoint set --name timer_interrupt_handler")?;
    writeln!(file, "breakpoint set --name spurious_io_apic_interrupt_handler")?;
    writeln!(file, "breakpoint set --name spurious_local_apic_interrupt_handler")?;
    writeln!(file, "breakpoint set --name breakpoint")?;
    // writeln!(file, "continue")?;

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
fn setup_env() -> Result<(), std::io::Error> {
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

    let dir = "target/iso";
    let img = "nocciolo.img";

    _ = std::fs::create_dir(dir);
    std::fs::copy(env!("BIOS_PATH"), &format!("{dir}/{img}"))?;

    Ok(())
}

fn does_command_exist(name: &str) -> bool {
    which::which(name).is_ok()
}
