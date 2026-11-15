use little_weirdo::effects::{
    bitcrunch::{Bitcrunch, BitcrunchConfiguration},
    Effect,
};

#[test]
fn bitcrunch_applies_quantization_when_enabled() {
    let mut effect = Bitcrunch::new(BitcrunchConfiguration {
        enabled: true,
        ..BitcrunchConfiguration::default()
    });
    for sample in [0, 1, 255] {
        assert_eq!(effect.clock(sample), 0);
    }
    assert_eq!(effect.clock(256), 256);
    for sample in [511, -1, -256] {
        assert_eq!(effect.clock(sample), 256);
    }
    assert_eq!(effect.clock(-511), 0);
}

#[test]
fn bitcrunch_leaves_signal_unchanged_when_disabled() {
    let mut effect = Bitcrunch::new(BitcrunchConfiguration {
        enabled: false,
        ..BitcrunchConfiguration::default()
    });
    assert_eq!(effect.clock(1234), 1234);
    assert_eq!(effect.clock(-4321), -4321);
    assert_eq!(effect.clock(32767), 32767);
    assert_eq!(effect.clock(-32768), -32768);
}

#[test]
fn bitcrunch_uses_configured_quantization_and_hold_interval() {
    let mut effect = Bitcrunch::new(BitcrunchConfiguration {
        enabled: true,
        quantize_step: 512,
        sample_hold_interval: 2,
    });

    assert_eq!(effect.clock(300), 0);
    assert_eq!(effect.clock(300), 512);
}
