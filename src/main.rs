use clap::Parser;
use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// EchoSafe Checkpoint 2: Advanced Editing Engine
#[derive(Parser, Debug)]
#[command(author, version, about = "EchoSafe - Audio Slicer & Advanced Processor")]
struct Args {
    /// [Library: clap] Input audio file path (.mp3, .aac, .m4a)
    #[arg(short, long)]
    input: String,

    /// [Library: clap] Output WAV file path
    #[arg(short, long)]
    output: String,

    /// [CP1: Volume Scaling] Volume multiplier (1.0 is original)
    #[arg(short, long, default_value_t = 1.0)]
    volume: f32,

    /// [CP1: Clipping] Start time in seconds
    #[arg(long)]
    start: Option<f32>,

    /// [CP1: Clipping] End time in seconds
    #[arg(long)]
    end: Option<f32>,

    /// [CP2: Fade] Fade duration in seconds (for both in and out)
    #[arg(long, default_value_t = 0.5)]
    fade: f32,

    /// [CP2: Speed] Playback speed multiplier (e.g., 2.0 is 2x faster)
    #[arg(long, default_value_t = 1.0)]
    speed: f32,

    /// [CP2: Filtering] Enable simple low-pass filter to smooth high-frequency noise
    #[arg(long, default_value_t = false)]
    lowpass: bool,
}

fn main() {
    let args = Args::parse();

    // 1. [Symphonia] Open and Probe
    let src = File::open(&args.input).expect("Failed to open input file");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    
    if args.input.ends_with(".mp3") { 
        hint.with_extension("mp3"); 
    } else if args.input.ends_with(".m4a") || args.input.ends_with(".mp4") {
        hint.with_extension("mov"); 
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .expect("Unsupported format");

    let mut format = probed.format;
    let track = format.tracks().iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .expect("No audio track found");

    // 2. [Symphonia] Initialize Decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .expect("Failed to create decoder");

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let mut all_samples: Vec<f32> = Vec::new();

    println!("Decoding: {} ...", args.input);

    // 3. Decoding Loop
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id { continue; }
        match decoder.decode(&packet) {
            Ok(AudioBufferRef::F32(buf)) => {
                for &sample in buf.chan(0) {
                    all_samples.push(sample);
                }
            }
            _ => {} 
        }
    }

    // 4. [CP1: Clipping] Calculate indices
    let start_idx = (args.start.unwrap_or(0.0) * sample_rate as f32) as usize;
    let mut end_idx = (args.end.unwrap_or(all_samples.len() as f32 / sample_rate as f32) * sample_rate as f32) as usize;
    
    if start_idx >= all_samples.len() {
        panic!("Start time is beyond file duration!");
    }
    if end_idx > all_samples.len() {
        end_idx = all_samples.len();
    }

    // Slice and initial volume scaling
    let mut processed: Vec<f32> = all_samples[start_idx..end_idx]
        .iter()
        .map(|&s| (s * args.volume))
        .collect();

    // 5. [CP2: Time Stretching] Simple Resampling
    if (args.speed - 1.0).abs() > 0.001 {
        println!("Applying Time Stretching: {}x speed", args.speed);
        let mut resampled = Vec::new();
        let mut pos = 0.0;
        while (pos as usize) < processed.len() {
            resampled.push(processed[pos as usize]);
            pos += args.speed; // Skip samples for speed-up, repeat for slow-down
        }
        processed = resampled;
    }

    // 6. [CP2: Frequency Filtering] Simple Low-Pass (Moving Average)
    if args.lowpass {
        println!("Applying Low-Pass Filter...");
        for i in 1..processed.len() {
            // A simple smoothing filter to reduce high-frequency content
            processed[i] = (processed[i] + processed[i-1]) / 2.0;
        }
    }

    // 7. [CP2: Fade-in/out]
    let fade_samples = (args.fade * sample_rate as f32) as usize;
    let total_len = processed.len();
    if fade_samples > 0 && total_len > fade_samples * 2 {
        println!("Applying Fade-in and Fade-out...");
        for i in 0..fade_samples {
            let ratio = i as f32 / fade_samples as f32;
            // Apply fade in at the beginning
            processed[i] *= ratio;
            // Apply fade out at the end
            processed[total_len - 1 - i] *= ratio;
        }
    }

    // Final safety clamp to prevent hardware-level clipping
    let final_samples: Vec<f32> = processed.iter().map(|&s| s.clamp(-1.0, 1.0)).collect();

    // 8. [Library: hound] Export to WAV
    let spec = hound::WavSpec {
        channels: 1, 
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(&args.output, spec)
        .expect("Failed to create WAV writer");
        
    for sample in final_samples {
        writer.write_sample(sample).unwrap();
    }

    println!("Success! File saved to: {}", args.output);
}