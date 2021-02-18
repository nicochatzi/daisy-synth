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

    let _audio_interface = audio_interface.start(move |_fs, block| {
        graph.process();
        for frame in block {
            let sample = outputs.out.dequeue().unwrap();
            *frame = (sample, sample);
        }
    });

    // - usart1 interrupt -----------------------------------------------------

    let mut midi_parser = midi::Parser::new();

    let (mut freq, mut fm, mut amp) = (inputs.freq, inputs.fm, inputs.amp);

    midi_interface
        .start(|byte| {
            midi_parser.rx(byte, |_channel, message| match message {
                Message::NoteOn { note, velocity } => {
                    let _ = freq.enqueue(convert::pitch::from_midi(note));
                }
                Message::NoteOff { note, velocity } => {}
                Message::ControlChange { index, value } => {}
                Message::ProgramChange { value } => {}
            });
        })
        .unwrap();

    // - main loop ------------------------------------------------------------

    // let mut knobs = (
    //     board.pins.SEED_15.into_analog(),
    //     board.pins.SEED_16.into_analog(),
    //     board.pins.SEED_17.into_analog(),
    //     board.pins.SEED_18.into_analog(),
    // );

    let one_second = board.clocks.sys_ck().0;

    loop {
        // if let Ok(data) = board.adc1.read(&mut knobs.0) {
        //     let data: u32 = data;
        //     let val = (1. + (data as f32 / board.adc1.max_sample() as f32)) * 64.;
        //     if let Ok(_) = freq.enqueue(val) {}
        // }
        // if let Ok(data) = board.adc1.read(&mut knobs.1) {
        //     let data: u32 = data;
        //     let val = (1. + (data as f32 / board.adc1.max_sample() as f32)) * 8.;
        //     let _ = fm.enqueue(8.0);
        // }
        // if let Ok(data) = board.adc1.read(&mut knobs.1) {
        //     let data: u32 = data;
        //     let freq = ((data as f32 / board.adc1.max_sample() as f32) + 1.0) * 64.0;
        //     if let Ok(_) = inputs.amp.enqueue(freq) {}
        // }
        asm::delay(10_000);
    }
}
