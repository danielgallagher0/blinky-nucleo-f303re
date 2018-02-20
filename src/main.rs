// examples/blinky.rs

#![deny(warnings)]
#![feature(const_fn)]
#![feature(used)]
#![feature(proc_macro)]
#![no_std]

extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate stm32f303x_e;

use cortex_m::peripheral::syst::SystClkSource;
use rtfm::{app, Threshold};
use stm32f303x_e::{GPIOA, GPIOC, EXTI};

const MAX_LEVEL: usize = 8; // Must be a power of 2

app! {
    device: stm32f303x_e,
    resources: {
        static LEVEL: usize = 0;
        static MODE: usize = 1;
        static IGNORE_BUTTON: usize = 0;
    },
    tasks: {
        EXTI15_10: {
            path: next_mode,
            resources: [MODE, IGNORE_BUTTON],
        },
        SYS_TICK: {
            path: periodic,
            resources: [LEVEL, MODE, IGNORE_BUTTON],
        },
    },
}

// INITIALIZATION PHASE
fn init(mut p: init::Peripherals, _r: init::Resources) {
    // Configure the PA5 pin as output pin, and PC13 as input
    p.device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
    p.device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
    p.device.GPIOA.moder.modify(
        |_, w| unsafe { w.moder5().bits(0b01) },
    );
    p.device.GPIOC.moder.modify(
        |_, w| unsafe { w.moder13().bits(0b00) },
    );

    // configure the system timer to generate one interrupt every one-eighth second
    p.core.SYST.set_clock_source(SystClkSource::Core);
    p.core.SYST.set_reload(1_000_000); // 0.125s
    p.core.SYST.enable_interrupt();
    p.core.SYST.enable_counter();

    p.core.NVIC.enable(
        stm32f303x_e::interrupt::Interrupt::EXTI15_10,
    );

    // Connect GPIOC13 to EXTI13 interrupt
    p.device.SYSCFG.exticr4.modify(|_, w| unsafe {
        w.exti13().bits(0b010)
    });

    // Enable the external interrupt for the push button on rise
    p.device.EXTI.imr1.modify(|_, w| w.mr13().set_bit());
    p.device.EXTI.emr1.modify(|_, w| w.mr13().set_bit());
    p.device.EXTI.rtsr1.modify(|_, w| w.tr13().set_bit());
}

// IDLE LOOP
fn idle() -> ! {
    // Sleep
    loop {
        rtfm::wfi();
    }
}

fn periodic(_t: &mut Threshold, mut r: SYS_TICK::Resources) {
    *r.LEVEL = (*r.LEVEL + 1) & (MAX_LEVEL - 1);

    if (*r.LEVEL & *r.MODE) > 0 {
        unsafe {
            (*GPIOA::ptr()).bsrr.write(|w| w.bs5().set_bit());
        }
    } else {
        unsafe {
            (*GPIOA::ptr()).bsrr.write(|w| w.br5().set_bit());
        }
    }

    if *r.IGNORE_BUTTON > 0 {
        *r.IGNORE_BUTTON -= 1;
    }
}

fn next_mode(_t: &mut Threshold, mut r: EXTI15_10::Resources) {
    if *r.IGNORE_BUTTON > 0 {
        return;
    }

    unsafe {
        if (*GPIOC::ptr()).idr.read().idr13().bit_is_set() {
            return;
        }
        (*EXTI::ptr()).pr1.reset();
    }

    *r.IGNORE_BUTTON = 4;
    *r.MODE *= 2;
    if *r.MODE >= MAX_LEVEL {
        *r.MODE = 1;
    }
}
