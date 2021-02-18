#[macro_use]
use rume::*;

#[derive(Debug, Clone)]
pub struct Delay {
    pub input: (DelaySampleInput, DelayTimeInput),
    pub output: DelaySampleOutput,
    sample: f32,
    delay_ticks: f32,
    sample_rate: u32,
    memory: [f32; 44_100],
    read_idx: f32,
    write_idx: usize,
    buffer_size: usize,
}

impl Default for Delay {
    fn default() -> Delay {
        Delay {
            input: (DelaySampleInput, DelayTimeInput),
            output: DelaySampleOutput,
            sample: 0.0,
            delay_ticks: 0.0,
            sample_rate: 44_100,
            memory: [0.0; 44_100],
            read_idx: 0.0,
            write_idx: 0,
            buffer_size: 0,
        }
    }
}

input! { Delay, DelayTimeInput,
    |proc: &mut Delay, time_ms: f32| {
        proc.delay_ticks = (time_ms * 0.001) * proc.sample_rate as f32;
        proc.buffer_size = proc.memory.len();
    }
}

input! { Delay, DelaySampleInput,
    |proc: &mut Delay, sample: f32| {
        proc.sample = sample;
    }
}

output! { Delay, DelaySampleOutput,
    |proc: &mut Delay| -> f32 {
        proc.sample
    }
}

#[inline(always)]
fn lerp(a: f32, b: f32, w: f32) -> f32 {
    a + w * (b - a)
}

impl Processor for Delay {
    fn prepare(&mut self, data: AudioConfig) {
        self.sample_rate = data.sample_rate as u32;
    }

    fn process(&mut self) {
        let buffer_size = self.memory.len();

        self.memory[self.write_idx] = self.sample;
        self.write_idx = (self.write_idx + 1) % buffer_size;
        self.read_idx = (self.write_idx as f32 - self.delay_ticks) % buffer_size as f32;

        let read_idx_0 = self.read_idx as usize;
        let read_idx_1 = (read_idx_0 + 1) % buffer_size;

        let wet = lerp(
            self.memory[read_idx_0],
            self.memory[read_idx_1],
            self.read_idx % 1.0,
        );

        self.sample = lerp(self.sample, wet, 0.5);
    }
}

#[rume::processor]
pub struct Distortion {
    #[sample]
    sample: f32,

    #[input]
    amount: f32,
}

impl Processor for Distortion {
    fn prepare(&mut self, _: AudioConfig) {}

    #[inline(always)]
    fn process(&mut self) {
        self.sample = (self.amount * self.sample).tanh();
    }
}

#[rume::processor]
pub struct Sine {
    #[input]
    frequency: f32,

    #[input]
    amplitude: f32,

    #[input]
    amount: f32,

    #[output]
    sample: f32,

    phase: [f32; 2],
    inv_sample_rate: f32,
}

impl Processor for Sine {
    fn prepare(&mut self, data: AudioConfig) {
        self.inv_sample_rate = 1.0 / data.sample_rate as f32;
    }

    fn process(&mut self) {
        const TWO_PI: f32 = 2.0_f32 * core::f32::consts::PI;

        let increment = TWO_PI * self.frequency * self.inv_sample_rate;
        self.phase[0] = (self.phase[0] + increment) % TWO_PI;
        self.sample = self.phase[0].sin();

        let increment = TWO_PI * self.frequency * self.inv_sample_rate * self.sample * self.amount;
        self.phase[1] = (self.phase[1] + increment) % TWO_PI;
        self.sample += self.phase[1].sin();

        self.sample *= self.amplitude;
    }
}

#[derive(Debug, Clone)]
enum EnvelopeState {
    Off = 0,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvelopeState {
    fn default() -> EnvelopeState {
        EnvelopeState::Off
    }
}

#[rume::processor]
pub struct Envelope {
    #[output]
    amplitude: f32,

    sample_rate: f32,
    state: EnvelopeState,

    #[input]
    attack_delta: f32,
    #[input]
    decay_delta: f32,
    #[input]
    sustain_level: f32,
    #[input]
    release_delta: f32,

    #[input]
    note_on: f32,
    #[input]
    note_off: f32,
}

impl Processor for Envelope {
    fn prepare(&mut self, data: rume::AudioConfig) {
        self.sample_rate = data.sample_rate as f32;
    }

    fn process(&mut self) {
        if self.note_on >= 1.0 {
            self.state = EnvelopeState::Attack;
            self.note_on = 0.0;
        }

        if self.note_off >= 1.0 {
            self.state = EnvelopeState::Release;
            self.note_off = 0.0;
        }

        match self.state {
            EnvelopeState::Attack => {
                self.amplitude += self.attack_delta;
                if self.amplitude >= 1.0 {
                    self.amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.amplitude -= self.decay_delta;
                if self.amplitude <= self.sustain_level {
                    if self.amplitude <= 0.0 {
                        self.state = EnvelopeState::Off;
                    } else {
                        self.amplitude = self.sustain_level;
                        self.state = EnvelopeState::Sustain;
                    }
                }
            }
            EnvelopeState::Sustain => {
                self.amplitude = self.sustain_level;
            }
            EnvelopeState::Release => {
                self.amplitude -= self.release_delta;
                if self.amplitude <= 0.0 {
                    self.amplitude = 0.0;
                    self.state = EnvelopeState::Off;
                }
            }
            EnvelopeState::Off => {
                self.amplitude = 0.0;
            }
        }
    }
}
graph! {
    inputs: {
        note_on: { kind: trigger },
        note_off: { kind: trigger },
        freq: { init: 220.0, range: 16.0..880.0, smooth: 10 },
        fm_amt: { init: 2.0, range: 0.01..16.0, smooth: 10 },
        dist_amt: { init: 0.2, range: 0.01..0.99, smooth: 10 },
        attack: { init: 1.0, range: 0.01..10.0, smooth: 10 },
        decay: { init: 2.0, range: 0.01..10.0, smooth: 10 },
        sustain: { init: 2.0, range: 0.01..10.0, smooth: 10 },
        release: { init: 2.0, range: 0.01..10.0, smooth: 10 },
        val: { init: 125.0, range: 10.0..2000.0, smooth: 10 },
    },
    outputs: {
        audio_out,
    },
    processors: {
        sine: Sine::default(),
        dist: Distortion::default(),
        env: Envelope::default(),
        dly: Delay::default(),
    },
    connections: {
        freq.output    ->  sine.input.0,
        env.output     ->  sine.input.1,
        fm_amt.output  ->  sine.input.2,

        attack.output   ->  env.input.0,
        decay.output    ->  env.input.1,
        sustain.output  ->  env.input.2,
        release.output  ->  env.input.3,

        note_on.output  ->  env.input.4,
        note_off.output ->  env.input.5,

        sine.output     ->  dist.input.0,
        dist_amt.output ->  dist.input.1,
        dist.output     ->  dly.input.0,
        val.output      ->  dly.input.1,
        dly.output      ->  audio_out.input,
    }
}
