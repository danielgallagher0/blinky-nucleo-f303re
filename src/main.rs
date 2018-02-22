// examples/blinky.rs

#![deny(warnings)]
#![feature(const_fn)]
#![feature(used)]
#![feature(proc_macro)]
#![no_std]

extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate stm32f30x;

use cortex_m::peripheral::syst::SystClkSource;
use rtfm::{app, Threshold};
use stm32f30x::{GPIOA, GPIOC, EXTI};

const MAX_LEVEL: usize = 8; // Must be a power of 2
const MAX_MODE: usize = MAX_LEVEL / 2;

app! {
    device: stm32f30x,
    resources: {
        static LEVEL: usize = 0;
        static MODE: usize = MAX_MODE;
        static IGNORE_BUTTON: usize = 0;
    },
    tasks: {
        EXTI15_10: {
            path: next_mode,
            resources: [MODE, IGNORE_BUTTON],
        },
        SYS_TICK: {
            path: maybe_toggle,
            resources: [LEVEL, MODE, IGNORE_BUTTON],
        },
    },
}

// INITIALIZATION PHASE
fn init(mut p: init::Peripherals, _r: init::Resources) {
    // Configure the PA5 pin as output (user LED)
    p.device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
    p.device.GPIOA.moder.modify(
        |_, w| w.moder5().output()
    );

    // Configure the PC13 in as input (user button)
    p.device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
    p.device.GPIOC.moder.modify(
        |_, w| w.moder13().input()
    );
    p.device.GPIOC.pupdr.modify(|_, w| unsafe {
        w.pupdr13().bits(0b01)  // Pull-up
    });

    // Configure the system timer to generate one interrupt every one-eighth
    // second, so we can fully blink the LED up to 4 times per second.
    p.core.SYST.set_clock_source(SystClkSource::Core);
    p.core.SYST.set_reload(1_000_000); // 0.125s
    p.core.SYST.enable_interrupt();
    p.core.SYST.enable_counter();

    // Enable the EXTI13 interrupt
    p.core.NVIC.enable(
        stm32f30x::interrupt::Interrupt::EXTI15_10,
    );

    // Connect GPIOC13 (user button) to EXTI13 interrupt
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

fn maybe_toggle(_t: &mut Threshold, mut r: SYS_TICK::Resources) {
    *r.LEVEL = (*r.LEVEL + 1) & (MAX_LEVEL - 1);

    // Turn the LED on iff the mode bit is on
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
    // We've handled the interrupt
    unsafe {
        (*EXTI::ptr()).pr1.reset();
    }

    // Ignore a bouncing signal
    if *r.IGNORE_BUTTON > 0 {
        return;
    }

    unsafe {
        // PC13 is pulled high by default, so if it's set, the button is not
        // pressed.
        if (*GPIOC::ptr()).idr.read().idr13().bit_is_set() {
            return;
        }
    }

    // We ignore a bouncing signal, so ignore any other activations for a few
    // cycles.  This value is decreased when the timer fires.
    *r.IGNORE_BUTTON = 4;

    // Increase the rate of the blinking LED, or cycle back from fastest to
    // slowest.
    *r.MODE /= 2;
    if *r.MODE == 0 {
        *r.MODE = MAX_MODE;
    }
}
