use clap::Parser; // clap
use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal}; // symphonia 
use symphonia::core::codecs::DecoderOptions; // symphonia 
use symphonia::core::formats::FormatOptions; // symphonia
use symphonia::core::io::MediaSourceStream; // symphonia
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// EchoSafe Checkpoint 1: Basic Editing Engine
#[derive(Parser, Debug)]
#[command(author, version, about = "EchoSafe - Audio Slicer & Volume Adjuster")]
struct Args {
    /// [Library: clap] Input audio file path (.mp3, .aac)
    #[arg(short, long)]
    input: String,

    /// [Library: clap] Output WAV file path
    #[arg(short, long)]
    output: String,

    /// [Checkpoint 1: Volume Scaling] Volume multiplier (1.0 is orignal volumn)
    #[arg(short, long, default_value_t = 1.0)]
    volume: f32,

    /// [Checkpoint 1: Clipping] Start time in seconds
    #[arg(long)]
    start: Option<f32>,

    /// [Checkpoint 1: Clipping] End time in seconds
    #[arg(long)]
    end: Option<f32>,
}

fn main() {
    // [Library: clap] Parse command line arguments into Args struct
    let args = Args::parse();

    // 1. [Library: symphonia] Open and Probe for Multi-format Support (MP3/AAC/ALAC)
    let src = File::open(&args.input).expect("Failed to open input file");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    
    if args.input.ends_with(".mp3") { 
        hint.with_extension("mp3"); 
    } else if args.input.ends_with(".m4a") || args.input.ends_with(".mp4") {
        hint.with_extension("mov"); 
    }

    // [Library: symphonia] Automatically detect container format
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .expect("Unsupported format");

    let mut format = probed.format;
    
    let track = format.tracks().iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .expect("No audio track found");

    // 2. [Library: symphonia] Initialize Decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .expect("Failed to create decoder");

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let mut all_samples: Vec<f32> = Vec::new();

    println!("Decoding: {} ...", args.input);

    // 3. Decoding Loop: Convert compressed audio to raw PCM (F32)
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id { continue; }
        match decoder.decode(&packet) {
            Ok(AudioBufferRef::F32(buf)) => {
                // Collect decoded samples into a single vector

                for &sample in buf.chan(0) {
                    all_samples.push(sample);
                }
            }
            _ => {} // Skip non-F32 formats for now (CP1 simplification)
        }
    }

    // 4. [Checkpoint 1: Clipping & Volume Scaling] Core Algorithms
    
    // Calculate indices based on sample rate: index = time * sample_rate (Hz)
    let start_idx = (args.start.unwrap_or(0.0) * sample_rate as f32) as usize;
    let mut end_idx = (args.end.unwrap_or(all_samples.len() as f32 / sample_rate as f32) * sample_rate as f32) as usize;
    
    // Bound checking to prevent program crash (Rust Safety)
    if start_idx >= all_samples.len() {
        panic!("Start time is beyond file duration!");
    }
    if end_idx > all_samples.len() {
        end_idx = all_samples.len();
    }

    println!("Processing: Slicing from index {} to {}", start_idx, end_idx);

    // Perform Volume Scaling and Clipping in one pass
    let processed_samples: Vec<f32> = all_samples[start_idx..end_idx]
        .iter()
        .map(|&s| (s * args.volume).clamp(-1.0, 1.0)) // Apply volume & prevent clipping
        .collect();

    // 5. [Library: hound] Export to High-Fidelity WAV
    let spec = hound::WavSpec {
        channels: 1, 
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(&args.output, spec)
        .expect("Failed to create WAV writer");
        
    for sample in processed_samples {
        writer.write_sample(sample).unwrap();
    }

    println!("File saved: {}", args.output);
}
