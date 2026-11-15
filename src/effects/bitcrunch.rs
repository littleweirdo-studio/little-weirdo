//! Bitcrunch (chip-tune/8 bit) effect

use crate::effects::Effect;
use serde::{Deserialize, Serialize};

// 16-bit audio is reduced to 8-bit steps by quantizing to a 256-sample grid.
const QUANTIZE_STEP: i32 = 256;
const SAMPLE_HOLD_INTERVAL: u8 = 4;

// Effect settings: only toggles whether the crunching is active.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct BitcrunchConfiguration {
    pub enabled: bool,
}

// Bitcrunch effect instance storing its runtime configuration.
pub struct Bitcrunch {
    pub config: BitcrunchConfiguration,
    held_sample: i16,
    samples_until_update: u8,
}

impl Bitcrunch {
    pub fn new(config: BitcrunchConfiguration) -> Self {
        Bitcrunch {
            config,
            held_sample: 0,
            samples_until_update: 0,
        }
    }

    // Replace the current settings without creating a new effect instance.
    pub fn reload(&mut self, config: BitcrunchConfiguration) {
        self.config = config;
        self.samples_until_update = 0;
    }
}

impl Effect for Bitcrunch {
    fn clock(&mut self, sample: i16) -> i16 {
        if !self.config.enabled {
            self.samples_until_update = 0;
            return sample;
        }

        if self.samples_until_update == 0 {
            let quantized = (sample as i32 / QUANTIZE_STEP) * QUANTIZE_STEP;
            self.held_sample = quantized.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            self.samples_until_update = SAMPLE_HOLD_INTERVAL - 1;
        } else {
            self.samples_until_update -= 1;
        }

        self.held_sample
    }
}
