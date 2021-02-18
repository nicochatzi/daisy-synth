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

    let (mut graph, mut inputs, mut outputs) = dsp::build();

    // - usart1 interrupt -----------------------------------------------------

    let mut midi_parser = midi::Parser::new();

    // let (mut freq, mut fm, mut amp) = (inputs.freq, inputs.fm, inputs.amp);

    midi_interface
        .start(|byte| {
            led_user.on();
            // if was_high {
            //     led_user.off();
            // } else {
            // }
            // midi_parser.rx(byte, |_channel, message| match message {
            //     Message::NoteOn { note, velocity } => {
            //         let _ = freq.enqueue(convert::pitch::from_midi(note));
            //         led_user.on();
            //     }
            //     Message::NoteOff { note, velocity } => {
            //         led_user.off();
            //     }
            //     Message::ControlChange { index, value } => {}
            //     Message::ProgramChange { value } => {}
            // });
        })
        .unwrap();

    // - audio callback -------------------------------------------------------

    graph.prepare(rume::AudioConfig {
        sample_rate: daisy::audio::FS.0 as usize,
        buffer_size: daisy::audio::BLOCK_LENGTH,
        num_channels: 2,
    });

    audio_interface.start(move |_fs, block| {
        graph.process();
        for frame in block {
            let sample = outputs.audio_out.dequeue().unwrap();
            *frame = (sample, sample);
        }
    });

    // - adc ------------------------------------------------------------------

    // switch adc_ker_ck_input multiplexer to per_ck
    board.peripheral.kernel_adc_clk_mux(AdcClkSel::PER);

    let cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    let mut delay = Delay::new(cp.SYST, board.clocks);
    let mut adc1 =
        adc::Adc::adc1(dp.ADC1, &mut delay, board.peripheral.ADC12, &board.clocks).enable();
    adc1.set_resolution(adc::Resolution::SIXTEENBIT);

    let gpioc = dp.GPIOC.split(board.peripheral.GPIOC);
    let mut adc1_channel_4 = gpioc.pc4.into_analog(); // pot 1
    let mut adc1_channel_10 = gpioc.pc0.into_analog(); // pot 2

    // - main loop ------------------------------------------------------------

    let _ = inputs.note_on.enqueue(1.0);
    loop {
        let val = {
            let val: u32 = adc1.read(&mut adc1_channel_10).unwrap();
            880. - (val as f32 * (880. / 65_535.))
        };
        let _ = inputs.freq.enqueue(val);

        // let val = {
        //     let val: u32 = adc1.read(&mut adc1_channel_4).unwrap();
        //     (val as f32 * (0.9 / 65_535.))
        // };
        // let _ = amp.enqueue(val);

        cortex_m::asm::wfi();
    }
}
