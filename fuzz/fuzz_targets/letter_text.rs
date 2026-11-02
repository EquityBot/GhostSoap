#![no_main]
//! The letter renderer and the line wrapper, on any text and width.

use std::collections::BTreeMap;

use ghostsoap::core::letters::{render, wrap};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (u8, &str)| {
    let (width, text) = input;
    let width = usize::from(width % 100) + 1;
    for line in wrap(text, width).lines() {
        // Only a single word may run past the width.
        assert!(
            line.chars().count() <= width || !line.contains(' '),
            "{line:?} is over {width}"
        );
    }
    let values = BTreeMap::from([("name", Some("Alex {{name}}".to_owned())), ("gone", None)]);
    if let Ok(out) = render(text, &values) {
        // Rendering keeps or drops whole lines and never adds one, though a value holds `{{`.
        assert!(
            out.lines().count() <= text.lines().count().max(1),
            "{out:?}"
        );
    }
});
