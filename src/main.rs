const USAGE: &str = "Pass sf2 file as the first arg and midi device name as the second one";

use std::{
    fs::File,
    process::exit,
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, BufferSize, StreamConfig};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::MidiInput;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

fn main() {
    let sf_file = std::env::args().nth(1).unwrap_or_else(|| {
        println!("{USAGE}");
        exit(1);
    });

    // Load the SoundFont.
    let mut sf2 = File::open(sf_file).unwrap();
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());

    // Create the synthesizer.
    let settings = SynthesizerSettings::new(44100);
    let mut synthesizer = Synthesizer::new(&sound_font, &settings).unwrap();

    let (midi_sender, midi_receiver) = channel();

    // MIDI
    let midi_in = MidiInput::new("riano").unwrap();
    let in_ports = midi_in.ports();
    let device_to_search = std::env::args().nth(2);

    println!("Discovered MIDI devices:");
    for p in &in_ports {
        println!("{:?}", midi_in.port_name(p))
    }

    let midi_device = match device_to_search {
        None => {
            println!("{USAGE}");
            exit(1);
        }
        Some(arg_p) => in_ports
            .iter()
            .find(|p| midi_in.port_name(p).unwrap().contains(&arg_p))
            .unwrap_or_else(|| panic!("MIDI device {arg_p:?} not found")),
    };

    println!("Connecting to {}", midi_in.port_name(midi_device).unwrap());

    let _conn = midi_in
        .connect(
            midi_device,
            "reading midi",
            move |_t, m, _| {
                let (m, _len) = MidiMsg::from_midi(m).unwrap();
                if let MidiMsg::ChannelVoice { msg, .. } = m {
                    midi_sender.send(msg).unwrap();
                }
            },
            (),
        )
        .unwrap();

    // Audio stream
    let host = cpal::default_host();
    let dev = host.default_output_device().unwrap();
    let mut conf: StreamConfig = dev.default_output_config().unwrap().into();
    conf.buffer_size = BufferSize::Fixed(512);
    let stream = dev
        .build_output_stream(
            &conf,
            move |d: &mut [f32], _| {
                while let Ok(msg) = midi_receiver.try_recv() {
                    match msg {
                        ChannelVoiceMsg::NoteOn { note, velocity } => {
                            synthesizer.note_on(0, note as i32, velocity as i32);
                        }
                        ChannelVoiceMsg::NoteOff { note, .. } => {
                            synthesizer.note_off(0, note as i32);
                        }
                        _ => {}
                    }
                }
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
