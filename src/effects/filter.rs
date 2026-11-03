//! Low, high, band, and notch filters.
use crate::{effects::Effect, math::fixed_mul};
use serde::{Deserialize, Serialize};

const FIXED_ONE: i64 = 1 << 16;

/// Types of filters available.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum KindOfFilter {
    Low,
    High,
    Band,
    Notch,
}

/// Filter roll-off steepness.
#[derive(Copy, Clone, Serialize, Deserialize, Default)]
pub enum FilterSlope {
    /// One two-pole filter section (12 dB/octave).
    #[default]
    Db12,
    /// Two cascaded two-pole filter sections (24 dB/octave).
    Db24,
}

/// Configuration for the filter effect.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub cutoff_frequency: u16,
    pub resonance: u16,
    pub kind_of_filter: KindOfFilter,
    pub enabled: bool,
    /// Filter roll-off steepness; older JSON patch files default to 12 dB/octave.
    #[serde(default)]
    pub slope: FilterSlope,
}

/// Digital filter implementation using fixed-point state-variable sections.
pub struct Filter {
    pub config: FilterConfig,
    stages: [FilterStage; 2],
}

impl Effect for Filter {
    fn clock(&mut self, sample: i16) -> i16 {
        if self.config.enabled {
            self.filter(sample)
        } else {
            sample
        }
    }
}

impl Filter {
    /// Create a new filter with the given configuration.
    pub fn new(config: FilterConfig) -> Self {
        let mut filter = Self {
            config,
            stages: [FilterStage::default(); 2],
        };
        filter.prepare_filter();
        filter
    }

    /// Update filter configuration and recompute parameters.
    pub fn reload(&mut self, config: FilterConfig) {
        self.config = config;
        self.prepare_filter();
    }

    fn prepare_filter(&mut self) {
        let g = (self.config.cutoff_frequency as i64 * FIXED_ONE) / u16::MAX as i64;
        let resonance = self.config.resonance as i64 * FIXED_ONE / u16::MAX as i64;
        // Map resonance quadratically to Q=0.5..10 so moderate control values
        // produce an audible peak instead of concentrating it near the maximum.
        let resonance_squared = fixed_mul(resonance, resonance);
        let q = FIXED_ONE / 2 + fixed_mul(FIXED_ONE * 19 / 2, resonance_squared);
        let damping = FIXED_ONE * FIXED_ONE / q;
        let denominator = FIXED_ONE + fixed_mul(g, g + damping);
        let a1 = (FIXED_ONE * FIXED_ONE) / denominator;
        let a2 = fixed_mul(g, a1);
        let a3 = fixed_mul(g, a2);

        for stage in &mut self.stages {
            stage.a1 = a1;
            stage.a2 = a2;
            stage.a3 = a3;
            stage.damping = damping;
        }
    }

    fn filter(&mut self, sample: i16) -> i16 {
        let sections = match self.config.slope {
            FilterSlope::Db12 => 1,
            FilterSlope::Db24 => 2,
        };
        let mut output = sample as i64;
        for stage in self.stages.iter_mut().take(sections) {
            output = stage.process(output, self.config.kind_of_filter);
        }
        output.clamp(i16::MIN as i64, i16::MAX as i64) as i16
    }
}

#[derive(Copy, Clone, Default)]
struct FilterStage {
    ic1eq: i64,
    ic2eq: i64,
    a1: i64,
    a2: i64,
    a3: i64,
    damping: i64,
}

impl FilterStage {
    fn process(&mut self, input: i64, kind: KindOfFilter) -> i64 {
        let v3 = input.saturating_sub(self.ic2eq);
        let v1 = fixed_mul(self.a1, self.ic1eq).saturating_add(fixed_mul(self.a2, v3));
        let v2 = self
            .ic2eq
            .saturating_add(fixed_mul(self.a2, self.ic1eq))
            .saturating_add(fixed_mul(self.a3, v3));

        self.ic1eq = v1.saturating_mul(2).saturating_sub(self.ic1eq);
        self.ic2eq = v2.saturating_mul(2).saturating_sub(self.ic2eq);

        let high = input
            .saturating_sub(fixed_mul(self.damping, v1))
            .saturating_sub(v2);
        match kind {
            KindOfFilter::Low => v2,
            KindOfFilter::High => high,
            KindOfFilter::Band => v1,
            KindOfFilter::Notch => high.saturating_add(v2),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::{Filter, FilterConfig, FilterSlope, KindOfFilter};
    use crate::effects::Effect;
    extern crate std;

    fn config(cutoff_frequency: u16, resonance: u16, slope: FilterSlope) -> FilterConfig {
        FilterConfig {
            cutoff_frequency,
            resonance,
            kind_of_filter: KindOfFilter::Low,
            enabled: true,
            slope,
        }
    }

    fn sine_rms(filter: &mut Filter, frequency: f64) -> f64 {
        let mut sum_squares = 0.0;
        let sample_count = 16_384;
        for index in 0..sample_count {
            let input = (8_000.0 * (std::f64::consts::TAU * frequency * index as f64).sin()) as i16;
            let output = filter.clock(input) as f64;
            if index >= sample_count / 2 {
                sum_squares += output * output;
            }
        }
        (sum_squares / (sample_count / 2) as f64).sqrt()
    }

    #[test]
    fn disabled_filter_is_transparent() {
        let mut filter = Filter::new(FilterConfig {
            enabled: false,
            ..config(12_000, 40_000, FilterSlope::Db12)
        });
        assert_eq!(filter.clock(i16::MIN), i16::MIN);
        assert_eq!(filter.clock(12_345), 12_345);
        assert_eq!(filter.clock(i16::MAX), i16::MAX);
    }

    #[test]
    fn twenty_four_db_slope_attenuates_high_frequencies_more() {
        let mut twelve_db = Filter::new(config(8_000, 0, FilterSlope::Db12));
        let mut twenty_four_db = Filter::new(config(8_000, 0, FilterSlope::Db24));
        let frequency = 0.12;

        let twelve_db_rms = sine_rms(&mut twelve_db, frequency);
        let twenty_four_db_rms = sine_rms(&mut twenty_four_db, frequency);

        assert!(twenty_four_db_rms < twelve_db_rms * 0.75);
    }

    #[test]
    fn resonance_increases_output_near_cutoff() {
        let mut no_resonance = Filter::new(config(8_000, 0, FilterSlope::Db12));
        let mut resonant = Filter::new(config(8_000, u16::MAX, FilterSlope::Db12));
        let near_cutoff = 0.04;

        let unresonant_rms = sine_rms(&mut no_resonance, near_cutoff);
        let resonant_rms = sine_rms(&mut resonant, near_cutoff);

        assert!(resonant_rms > unresonant_rms * 1.5);
    }

    #[test]
    fn midi_reachable_resonance_produces_audible_peak_near_cutoff() {
        let mut no_resonance = Filter::new(config(8_000, 0, FilterSlope::Db12));
        let mut midi_max_resonance = Filter::new(config(8_000, 127 * 255, FilterSlope::Db12));
        let near_cutoff = 0.04;

        let unresonant_rms = sine_rms(&mut no_resonance, near_cutoff);
        let resonant_rms = sine_rms(&mut midi_max_resonance, near_cutoff);

        assert!(resonant_rms > unresonant_rms * 2.0);
    }

    #[test]
    fn extreme_resonance_and_cutoff_remain_bounded() {
        let mut filter = Filter::new(config(u16::MAX, u16::MAX, FilterSlope::Db24));
        for index in 0..32_768 {
            let sample = if index % 2 == 0 { i16::MAX } else { i16::MIN };
            let output = filter.clock(sample);
            assert!((i16::MIN..=i16::MAX).contains(&output));
        }
        assert!(filter.stages.iter().all(|stage| {
            stage.ic1eq.unsigned_abs() < 100_000_000 && stage.ic2eq.unsigned_abs() < 100_000_000
        }));
    }

    #[test]
    fn slope_is_independent_of_filter_kind() {
        let mut config = config(8_000, 0, FilterSlope::Db12);
        config.slope = FilterSlope::Db24;
        assert!(matches!(config.kind_of_filter, KindOfFilter::Low));
        assert!(matches!(config.slope, FilterSlope::Db24));
        config.slope = FilterSlope::Db12;
        assert!(matches!(config.kind_of_filter, KindOfFilter::Low));
    }
}
