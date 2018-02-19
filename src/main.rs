// examples/blinky.rs

#![feature(used)]
#![no_std]

// version = "0.2.0", default-features = false
extern crate cast;
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate cortex_m_semihosting;
extern crate stm32f303x_e;

use core::u16;

use cast::{u16, u32};
use cortex_m::asm;
use stm32f303x_e::Peripherals;

mod frequency {
    /// Frequency of APB1 bus (TIM7 is connected to this bus)
    pub const APB1: u32 = 8_000_000;
}

/// Timer frequency
const FREQUENCY: u32 = 1;

#[inline(never)]
fn main() {
    // Critical section, this closure is non-preemptable
    cortex_m::interrupt::free(|_cs| {
        // INITIALIZATION PHASE
        // Exclusive access to the peripherals
        let peripherals = Peripherals::take().unwrap();
        let gpioa = peripherals.GPIOA;
        let rcc = peripherals.RCC;
        let tim7 = peripherals.TIM7;

        // Power up the relevant peripherals
        rcc.ahbenr.modify(|_, w| w.iopaen().set_bit());
        rcc.apb1enr.modify(|_, w| w.tim7en().set_bit());

        // Configure the pin PA5 as an output pin
        gpioa.moder.modify(|_, w| unsafe { w.moder5().bits(0b01) });

        // Configure TIM7 for periodic timeouts
        let ratio = frequency::APB1 / FREQUENCY;
        let psc = u16((ratio - 1) / u32(u16::MAX)).unwrap();
        tim7.psc.write(|w| unsafe { w.psc().bits(psc) });
        let arr = u16(ratio / u32(psc + 1)).unwrap();
        tim7.arr.write(|w| unsafe { w.arr().bits(arr) });
        tim7.cr1.write(|w| w.opm().clear_bit());

        // Start the timer
        tim7.cr1.modify(|_, w| w.cen().set_bit());

        // APPLICATION LOGIC
        let mut state = false;
        loop {
            // Wait for an update event
            while !tim7.sr.read().uif().bit() {}

            // Clear the update event flag
            tim7.sr.modify(|_, w| w.uif().clear_bit());

            // Toggle the state
            state = !state;

            // Blink the LED
            if state {
                gpioa.bsrr.write(|w| w.bs5().set_bit());
            } else {
                gpioa.bsrr.write(|w| w.br5().set_bit());
            }
        }
    });
}

// This part is the same as before
#[allow(dead_code)]
#[used]
#[link_section = ".vector_table.interrupts"]
static INTERRUPTS: [extern "C" fn(); 240] = [default_handler; 240];

extern "C" fn default_handler() {
    asm::bkpt();
}
