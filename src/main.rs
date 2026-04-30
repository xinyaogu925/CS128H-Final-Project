use anyhow::{bail, Result};
use clap::Parser;
use realfft::RealFftPlanner;
use std::f32::consts::PI;
use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// --- Command Line Argument Configuration ---
// Defines the CLI structure for input paths and various audio processing flags.
#[derive(Parser, Debug)]
#[command(author, version, about = "EchoSafe - Audio Slicer, Spectral Processor & Security Suite")]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value_t = 1.0)]
    volume: f32,

    #[arg(long)]
    start: Option<f32>,

    #[arg(long)]
    end: Option<f32>,

    #[arg(long, default_value_t = 0.0)]
    fade: f32,

    #[arg(long, default_value_t = 1.0)]
    speed: f32,

    #[arg(long, default_value_t = 0.0)]
    pitch_shift: f32,

    #[arg(long)]
    remove_frequency: Option<f32>,

    #[arg(long, default_value_t = false)]
    lowpass: bool,

    #[arg(long, default_value_t = false)]
    highpass: bool,

    #[arg(long, default_value_t = false)]
    normalize: bool,

    #[arg(long, default_value_t = false)]
    visualize: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // --- 1. Audio Decoding ---
    // Loads the source file (MP3/M4A/MP4) and converts it into a raw floating-point sample vector.
    let (all_samples, sample_rate) = decode_audio(&args.input)?;
    let sample_rate_f = sample_rate as f32;

    // --- 2. Audio Slicing / Clipping ---
    // Calculates the start and end indices based on the provided timestamps.
    let start_idx = (args.start.unwrap_or(0.0) * sample_rate_f) as usize;
    let mut end_idx = (args.end.unwrap_or(all_samples.len() as f32 / sample_rate_f) * sample_rate_f) as usize;
    
    if start_idx >= all_samples.len() {
        bail!("Start time exceeds file duration!");
    }
    end_idx = end_idx.min(all_samples.len());

    let mut samples: Vec<f32> = all_samples[start_idx..end_idx].to_vec();

    // --- 3. Spectral Visualization ---
    // Performs FFT analysis and prints a frequency distribution bar chart to the terminal.
    if args.visualize {
        let freqs = compute_fft_and_get_frequencies(&samples, sample_rate);
        display_spectral_visualization(&freqs);
    }

    // --- 4. Gain Adjustment & Normalization ---
    // Adjusts volume and ensures the audio levels are consistent across the track.
    if (args.volume - 1.0).abs() > 0.001 {
        for sample in &mut samples { *sample *= args.volume; }
    }

    if args.normalize {
        samples = normalize_loudness(&samples);
    }

    // --- 5. Digital Signal Processing (DSP) Filters ---
    // Applies low-pass, high-pass, or notch filters to remove noise or shape the tone.
    if args.lowpass {
        samples = apply_lowpass_filter(&samples, sample_rate, 2000.0);
    }

    if args.highpass {
        samples = apply_highpass_filter(&samples, sample_rate, 80.0);
    }

    if let Some(freq) = args.remove_frequency {
        samples = apply_notch_filter(&samples, sample_rate, freq, 20.0);
    }

    // --- 6. Time and Pitch Manipulation ---
    // Changes the speed of playback or shifts the pitch semitones via resampling.
    if (args.pitch_shift).abs() > 0.01 {
        samples = apply_pitch_shift(&samples, args.pitch_shift);
    }

    if (args.speed - 1.0).abs() > 0.001 {
        samples = apply_time_stretch(&samples, args.speed);
    }

    // --- 7. Fading Algorithms ---
    // Smoothly ramps the volume at the beginning (Fade-in) and end (Fade-out).
    if args.fade > 0.0 {
        let fade_len = (args.fade * sample_rate_f) as usize;
        let len = samples.len();
        if len > fade_len * 2 {
            for i in 0..fade_len {
                let ratio = i as f32 / fade_len as f32;
                samples[i] *= ratio;
                samples[len - 1 - i] *= ratio;
            }
        }
    }

    // --- 8. Final Safety Check & Export ---
    // Clamps samples to prevent digital clipping and saves the result as a WAV file.
    for sample in &mut samples { *sample = sample.clamp(-1.0, 1.0); }
    export_to_wav(&samples, sample_rate as u32, &args.output)?;

    println!("✅ Processed: {} samples saved to {}", samples.len(), args.output);
    Ok(())
}

// --- Audio Decoding Engine ---
// Handles format probing and frame decoding using the Symphonia library.
fn decode_audio(input_path: &str) -> Result<(Vec<f32>, usize)> {
    let src = File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    
    if input_path.ends_with(".mp3") { hint.with_extension("mp3"); }
    else if input_path.ends_with(".m4a") || input_path.ends_with(".mp4") { hint.with_extension("mov"); }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

    let mut format = probed.format;
    let track = format.tracks().iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("No audio track found"))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100) as usize;
    let mut all_samples = Vec::new();

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id { continue; }
        if let Ok(AudioBufferRef::F32(buf)) = decoder.decode(&packet) {
            for &sample in buf.chan(0) { all_samples.push(sample); }
        }
    }
    Ok((all_samples, sample_rate))
}

// --- Low-pass Filter ---
// Reduces high-frequency content using a simple moving average window.
fn apply_lowpass_filter(samples: &[f32], sample_rate: usize, cutoff_hz: f32) -> Vec<f32> {
    let window_size = ((sample_rate as f32 / cutoff_hz).round() as usize).clamp(2, 50);
    let mut filtered = Vec::with_capacity(samples.len());
    for i in 0..samples.len() {
        let start = i.saturating_sub(window_size);
        let count = (i - start + 1) as f32;
        let sum: f32 = samples[start..=i].iter().sum();
        filtered.push(sum / count);
    }
    filtered
}

// --- High-pass Filter ---
// Removes low-frequency content by subtracting the low-passed signal from the original.
fn apply_highpass_filter(samples: &[f32], sample_rate: usize, cutoff_hz: f32) -> Vec<f32> {
    let lowpassed = apply_lowpass_filter(samples, sample_rate, cutoff_hz);
    samples.iter().zip(lowpassed.iter()).map(|(orig, low)| (orig - low).clamp(-1.0, 1.0)).collect()
}

// --- Notch Filter ---
// Targets and attenuates a very specific frequency band (useful for hum removal).
fn apply_notch_filter(samples: &[f32], sample_rate: usize, center_freq: f32, bandwidth: f32) -> Vec<f32> {
    let sr = sample_rate as f32;
    let omega = 2.0 * PI * center_freq / sr;
    let bw_rad = 2.0 * PI * bandwidth / sr;
    let r = 1.0 - bw_rad / 2.0;
    
    let a1 = -2.0 * r * omega.cos();
    let a2 = r * r;
    let b1 = -2.0 * omega.cos();
    
    let mut out = vec![0.0; samples.len()];
    for i in 2..samples.len() {
        out[i] = samples[i] + b1 * samples[i-1] + samples[i-2] - a1 * out[i-1] - a2 * out[i-2];
    }
    out
}

// --- Pitch Shifting ---
// Changes the perceived note height by calculating a resampling factor.
fn apply_pitch_shift(samples: &[f32], semitones: f32) -> Vec<f32> {
    let factor = 2.0f32.powf(semitones / 12.0);
    apply_time_stretch(samples, factor)
}

// --- Time Stretching (Resampling) ---
// Adjusts playback speed through linear interpolation.
fn apply_time_stretch(samples: &[f32], speed: f32) -> Vec<f32> {
    let new_len = (samples.len() as f32 / speed) as usize;
    (0..new_len).map(|i| {
        let pos = i as f32 * speed;
        let idx = pos as usize;
        let frac = pos - idx as f32;
        if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac) + samples[idx+1] * frac
        } else {
            samples.get(idx).cloned().unwrap_or(0.0)
        }
    }).collect()
}

// --- Loudness Normalization ---
// Analyzes RMS power and scales the entire signal to a target level.
fn normalize_loudness(samples: &[f32]) -> Vec<f32> {
    let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let gain = if rms > 0.001 { 0.2 / rms } else { 1.0 };
    samples.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect()
}

// --- FFT Analysis ---
// Computes Fast Fourier Transform with a Hann window to identify frequency magnitudes.
fn compute_fft_and_get_frequencies(samples: &[f32], sample_rate: usize) -> Vec<(f32, f32)> {
    let n = samples.len().next_power_of_two().min(16384); 
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);
    let mut indata = vec![0.0; n];
    for (i, &s) in samples.iter().take(n).enumerate() {
        let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / (n as f32 - 1.0)).cos());
        indata[i] = s * window;
    }
    let mut outdata = r2c.make_output_vec();
    let _ = r2c.process(&mut indata, &mut outdata);
    
    outdata.iter().enumerate().map(|(i, c)| {
        let freq = (i as f32 * sample_rate as f32) / n as f32;
        (freq, c.norm())
    }).collect()
}

// --- Spectral Terminal Display ---
// Groups frequency data into Bass, Mid, and High bands for ASCII visualization.
fn display_spectral_visualization(freqs: &[(f32, f32)]) {
    let bands = [("Bass", 0.0, 250.0), ("Mid", 250.0, 4000.0), ("High", 4000.0, 20000.0)];
    println!("\n--- Spectral Analysis ---");
    for (name, low, high) in bands {
        let val: f32 = freqs.iter().filter(|(f, _)| *f >= low && *f < high).map(|(_, m)| m).sum();
        let bar = "█".repeat((val.min(50.0)) as usize);
        println!("{:<5} | {}", name, bar);
    }
}

// --- WAV Export ---
// Encodes the raw samples into a standard 32-bit floating point WAV format.
fn export_to_wav(samples: &[f32], sample_rate: u32, path: &str) -> Result<()> {
    let spec = hound::WavSpec { channels: 1, sample_rate, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples { writer.write_sample(s)?; }
    writer.finalize()?;
    Ok(())
}