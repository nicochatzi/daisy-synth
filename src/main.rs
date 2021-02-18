#![no_main]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use cortex_m::asm;
use cortex_m_rt::entry;
use panic_semihosting as _;

use daisy::led::Led;
use daisy_bsp as daisy;

use daisy::hal::prelude::*;

use daisy_bsp::hal;
use hal::rcc::rec::AdcClkSel;
use hal::{adc, delay::Delay};

use embedded_hal::digital::v2::OutputPin;
use hal::hal as embedded_hal;

use hal::{pac, prelude::*};

use rume::*;
mod dsp;
mod midi;
use midi::Message;

#[entry]
fn main() -> ! {
    // - board setup ----------------------------------------------------------

    let mut board = daisy::Board::take().unwrap();

    let mut led_user = board.leds.USER;
    let mut audio_interface = board.SAI1;
    let mut midi_interface = board.USART1;

    // - audio callback -------------------------------------------------------

    let (mut graph, mut inputs, mut outputs) = dsp::build();
    graph.prepare(rume::AudioConfig {
        sample_rate: daisy::audio::FS.0 as usize,
        buffer_size: daisy::audio::BLOCK_LENGTH,
        num_channels: 2,
    });

    audio_interface.start(move |_fs, block| {
        graph.process();
        for frame in block {
            let sample = outputs.out.dequeue().unwrap();
            *frame = (sample, sample);
        }
    });

    // - usart1 interrupt -----------------------------------------------------

    let mut midi_parser = midi::Parser::new();

    // midi_interface
    //     .start(|byte| {
    //         midi_parser.rx(byte, |_channel, message| {
    //             match message {
    //                 Message::NoteOn { note, velocity } => {
    //                     let _ = freq.enqueue(convert::pitch::from_midi(note));
    //                     led_user.on();
    //                 }
    //                 Message::NoteOff { note, velocity } => {
    //                     led_user.off();
    //                 }
    //                 Message::ControlChange { index, value } => {}
    //                 Message::ProgramChange { value } => {}
    //             }
    //         });
    //     })
    //     .unwrap();

    // - main loop ------------------------------------------------------------

    // - clocks ---------------------------------------------------------------

    // switch adc_ker_ck_input multiplexer to per_ck
    board.peripheral.kernel_adc_clk_mux(AdcClkSel::PER);

    // - adc ------------------------------------------------------------------

    let cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    let mut delay = Delay::new(cp.SYST, board.clocks);
    let mut adc1 =
        adc::Adc::adc1(dp.ADC1, &mut delay, board.peripheral.ADC12, &board.clocks).enable();
    adc1.set_resolution(adc::Resolution::SIXTEENBIT);

    let mut pot_1 = board.pins.SEED_15.into_analog();
    let mut pot_2 = board.pins.SEED_16.into_analog();
    let mut pot_3 = board.pins.SEED_21.into_analog();
    let mut pot_4 = board.pins.SEED_18.into_analog();

    // - main loop ------------------------------------------------------------

    let max_val_factor = 1. / 65536.0_f32;
    let normalise = move |val: u32| -> f32 { 1. - (val as f32 * max_val_factor) };

    loop {
        inputs
            .freq
            .enqueue(normalise(adc1.read(&mut pot_1).unwrap()) * 880.);

        inputs
            .amp_c
            .enqueue(normalise(adc1.read(&mut pot_2).unwrap()) * 1.);

        inputs
            .ratio
            .enqueue(normalise(adc1.read(&mut pot_3).unwrap()) * 4.);

        inputs
            .amp_m
            .enqueue(normalise(adc1.read(&mut pot_4).unwrap()) * 4.);

        cortex_m::asm::wfi();
    }
}
