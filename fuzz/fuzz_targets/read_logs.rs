//! Fuzz target: the readers of point 0 and of the specifications never panic,
//! and whatever they accept writes back to a text they read the same.

#![no_main]

use libfuzzer_sys::fuzz_target;
use monade_core::origin::Origin;
use monade_core::specs::Specs;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(o) = Origin::from_text(text) {
        assert_eq!(Origin::from_text(&o.to_text()), Ok(o));
    }
    if let Ok(s) = Specs::from_text(text) {
        assert_eq!(Specs::from_text(&s.to_text()), Ok(s));
    }
});
