// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::time::Duration;

use lazy_static::lazy_static;
use log::trace;
use spin::Mutex;

use x86_64::instructions::{
    hlt,
    interrupts::without_interrupts,
    port::{Port, PortWriteOnly},
};

use crate::interrupts::{apic::IOApic, TIMER};

lazy_static! {
    static ref CHANNEL0: Mutex<Port<u8>> = Mutex::new(Port::new(0x40));
    static ref MODE_COMMAND: Mutex<PortWriteOnly<u8>> = Mutex::new(PortWriteOnly::new(0x43));
}

const BASE_FREQUENCY: usize = 1193182;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Channel {
    Channel0 = 0b00,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum AccessMode {
    LatchCountValue = 0b00,
    LoByteOnly = 0b01,
    HiByteOnly = 0b10,
    LoAndHiByte = 0b11,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum OperatingMode {
    InterruptOnTerminalCount = 0b000,
    HardwareRetriggerableOneShot = 0b001,
    RateGenerator = 0b010,
    SquareWave = 0b011,
    SoftwareTriggeredStrobe = 0b100,
    HardwareTriggeredStrobe = 0b101,
    RateGenerator2 = 0b110,
    SquareWave2 = 0b111,
}

pub fn init() {
    write_mode_command(Channel::Channel0, AccessMode::LoAndHiByte, OperatingMode::SquareWave, false);
    set_frequency(1000);
}

pub fn sleep(s: Duration) {
    let ms = s.as_millis() as usize;
    if ms == 0 {
        return;
    }

    let end =  get_pit_uptime() + ms;
    trace!("Sleeping for {ms} milliseconds (until {end})...");

    while end != get_pit_uptime() {
        trace!("Halting");
        // Halting waits for the timer interrupt
        hlt();
    }
}

fn get_pit_uptime() -> usize {
    TIMER.lock().read()
}

#[allow(unused)]
fn read_count() -> u16 {
    without_interrupts(|| {
        write_mode_command(Channel::Channel0, AccessMode::LatchCountValue, OperatingMode::InterruptOnTerminalCount, false);

        let mut channel0 = CHANNEL0.lock();

        let lo = unsafe { channel0.read() } as u16;
        let hi = unsafe { channel0.read() } as u16;

        lo | (hi << 8)
    })
}

fn set_frequency(frequency: usize) {
    let frequency = BASE_FREQUENCY / frequency;
    debug_assert!(frequency <= (u16::MAX as usize), "invalid frequency: {frequency}");
    write_reload_count(frequency as u16)
}

fn write_reload_count(count: u16) {
    let lo = (count & 0xFF) as u8;
    let hi = ((count >> 8) & 0xFF) as u8;

    without_interrupts(|| {
        let mut channel0 = CHANNEL0.lock();

        unsafe {
            channel0.write(lo);
            channel0.write(hi);
        }
    });
}

fn write_mode_command(channel: Channel, access_mode: AccessMode, operating_mode: OperatingMode, bcd: bool) {
    let value = (channel as u8) << 6
              | (access_mode as u8) << 4
              | (operating_mode as u8) << 1
              | (bcd as u8);

    trace!("Write {value:x} ({value:b})");

    unsafe {
        MODE_COMMAND.lock().write(value);
    }
}
