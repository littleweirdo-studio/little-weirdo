//! Bitcrunch (chip-tune/8 bit) effect

use crate::effects::Effect;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct BitcrunchConfiguration {
    pub enabled: bool,
    #[serde(default = "default_quantize_step")]
    pub quantize_step: u16,
    #[serde(default = "default_sample_hold_interval")]
    pub sample_hold_interval: u8,
}

impl Default for BitcrunchConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            quantize_step: default_quantize_step(),
            sample_hold_interval: default_sample_hold_interval(),
        }
    }
}

fn default_quantize_step() -> u16 {
    256
}

fn default_sample_hold_interval() -> u8 {
    4
}

// Bitcrunch effect instance storing its runtime configuration.
pub struct Bitcrunch {
    pub config: BitcrunchConfiguration,
    held_sample: i16,
    sample_accumulator: i32,
    samples_accumulated: u8,
}

impl Bitcrunch {
    pub fn new(config: BitcrunchConfiguration) -> Self {
        Bitcrunch {
            config,
            held_sample: 0,
            sample_accumulator: 0,
            samples_accumulated: 0,
        }
    }

    // Replace the current settings without creating a new effect instance.
    pub fn reload(&mut self, config: BitcrunchConfiguration) {
        self.config = config;
        self.sample_accumulator = 0;
        self.samples_accumulated = 0;
    }
}

impl Effect for Bitcrunch {
    fn clock(&mut self, sample: i16) -> i16 {
        if !self.config.enabled {
            self.sample_accumulator = 0;
            self.samples_accumulated = 0;
            return sample;
        }

        self.sample_accumulator += sample as i32;
        self.samples_accumulated += 1;

        let sample_hold_interval = self.config.sample_hold_interval.max(1);
        if self.samples_accumulated == sample_hold_interval {
            let average = self.sample_accumulator / sample_hold_interval as i32;
            let quantize_step = self.config.quantize_step.max(1) as i32;
            let rounding_offset = if average >= 0 {
                quantize_step / 2
            } else {
                -(quantize_step / 2)
            };
            let quantized = ((average + rounding_offset) / quantize_step) * quantize_step;
            self.held_sample = quantized.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            self.sample_accumulator = 0;
            self.samples_accumulated = 0;
        }

        self.held_sample
    }
}
