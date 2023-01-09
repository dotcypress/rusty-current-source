#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use defmt_rtt as _;

use hal::analog::adc;
use hal::exti;
use hal::gpio::*;
use hal::power::*;
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use hal::timer::*;

const GATE_SATURATION: u16 = 3_000;
const TTL: usize = 20 * 60 * 3;
const MODES: [u16; 6] = [
    0,   // Off
    14,  // 1 mA
    65,  // 5 mA
    129, // 10 mA
    257, // 20 mA
    641, // 50 mA
];

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        mode: usize,
    }

    #[local]
    struct Local {
        ttl: usize,
        frame: usize,
        adc: adc::Adc,
        scb: stm32::SCB,
        exti: stm32::EXTI,
        pwr: Power,
        sense: PB7<Analog>,
        timer: Timer<stm32::TIM16>,
        pwm: pwm::PwmPin<hal::pac::TIM17, Channel1>,
        leds: [gpioa::PA<Output<PushPull>>; 5],
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let scb = ctx.core.SCB;
        let mut exti = ctx.device.EXTI;
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());

        let port_a = ctx.device.GPIOA.split(&mut rcc);
        let port_b = ctx.device.GPIOB.split(&mut rcc);

        let sense = port_b.pb7.into_analog();
        let mut adc = ctx.device.ADC.constrain(&mut rcc);
        adc.set_sample_time(adc::SampleTime::T_80);
        adc.set_precision(adc::Precision::B_12);
        adc.set_oversampling_shift(22);
        adc.set_oversampling_ratio(adc::OversamplingRatio::X_8);
        adc.oversampling_enable(true);

        let mut pwr = ctx.device.PWR.constrain(&mut rcc);
        pwr.clear_standby_flag();
        pwr.enable_wakeup_lane(WakeUp::Line2, SignalEdge::Rising);
        port_a
            .pa4
            .into_floating_input()
            .listen(SignalEdge::Rising, &mut exti);

        let pwm = ctx.device.TIM17.pwm(64.kHz(), &mut rcc);
        let mut pwm = pwm.bind_pin(port_b.pb9);
        let mode = 1;
        pwm.set_duty(MODES[mode]);
        pwm.enable();

        let mut timer = ctx.device.TIM16.timer(&mut rcc);
        timer.start(50.millis());
        timer.listen();

        let mut leds = [
            port_a.pa1.into_push_pull_output().downgrade(),
            port_a.pa2.into_push_pull_output().downgrade(),
            port_a.pa3.into_push_pull_output().downgrade(),
            port_a.pa5.into_push_pull_output().downgrade(),
            port_a.pa6.into_push_pull_output().downgrade(),
        ];
        leds[0].set_high().ok();

        port_a.pa7.into_push_pull_output_in_state(PinState::High);
        adc.calibrate();

        (
            Shared { mode },
            Local {
                adc,
                timer,
                exti,
                leds,
                pwr,
                scb,
                pwm,
                sense,
                ttl: TTL,
                frame: 0,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI4_15, local = [exti, pwm], shared = [mode])]
    fn touch(ctx: touch::Context) {
        let mut mode = ctx.shared.mode;
        mode.lock(|mode| {
            *mode = mode.saturating_add(1) % MODES.len();
            ctx.local.pwm.set_duty(MODES[*mode]);
        });
        ctx.local.exti.unpend(exti::Event::GPIO4);
    }

    #[task(binds = TIM16, local = [adc, frame, leds, timer, pwr, scb, sense, ttl], shared=[mode])]
    fn timer_tick(ctx: timer_tick::Context) {
        let timer_tick::LocalResources {
            adc,
            leds,
            frame,
            timer,
            pwr,
            scb,
            sense,
            ttl,
        } = ctx.local;

        let mut mode = ctx.shared.mode;
        let mode = mode.lock(|mode| *mode);
        let led_idx = mode.saturating_sub(1);

        let gate_voltage = adc.read_voltage(sense).unwrap_or(u16::MAX);
        let open_circuit = gate_voltage > GATE_SATURATION;

        *frame = frame.saturating_add(1);
        for (idx, led) in leds.iter_mut().enumerate() {
            let state = if idx == led_idx {
                if !open_circuit || *frame % 4 != 0 {
                    PinState::High
                } else {
                    PinState::Low
                }
            } else {
                PinState::Low
            };
            led.set_state(state).ok();
        }

        *ttl = if open_circuit {
            ttl.saturating_sub(1)
        } else {
            TTL
        };

        if mode == 0 || *ttl == 0 {
            pwr.clear_wakeup_flag(WakeUp::Line2);
            pwr.set_mode(PowerMode::LowPower(LowPowerMode::Shutdown));
            scb.set_sleepdeep();
        }

        timer.clear_irq();
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            rtic::export::wfi();
        }
    }
}
