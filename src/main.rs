use std::{fs::File, process::exit, sync::Arc, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

fn main() {
    let usage = "Pass sf2 file as the first arg";
    let sf_file = std::env::args().nth(1).unwrap_or_else(|| {
        println!("{usage}");
        exit(1);
    });

    // Load the SoundFont.
    let mut sf2 = File::open(sf_file).unwrap();
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());

    // Create the synthesizer.
    let settings = SynthesizerSettings::new(44100);
    let mut synthesizer = Synthesizer::new(&sound_font, &settings).unwrap();

    // Play some notes (middle C, E, G).
    synthesizer.note_on(0, 60, 100);
    synthesizer.note_on(0, 64, 100);
    synthesizer.note_on(0, 67, 100);

    // Audio stream
    let host = cpal::default_host();
    let dev = host.default_output_device().unwrap();
    let conf = dev.default_output_config().unwrap();
    let stream = dev
        .build_output_stream(
            &conf.into(),
            move |d: &mut [f32], _| {
                let mut left = vec![0.0; d.len() / 2];
                let mut right = vec![0.0; d.len() / 2];
                synthesizer.render(&mut left, &mut right);
                for (i, c) in d.chunks_exact_mut(2).enumerate() {
                    c[0] = left[i];
                    c[1] = right[i];
                }
            },
            |e| println!("{e}"),
            None,
        )
        .unwrap();
    stream.play().unwrap();

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
