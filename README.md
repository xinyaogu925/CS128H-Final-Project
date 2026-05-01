# CS128H-Final-Project
Project Proposal: EchoSafe - Advanced Interactive Audio Workstation

Group name: FrequencyX

Group members: Juan Fu, juanfu2, Xinyao Gu, xinyaog3

Project introduction:

EchoSafe is a high-performance, interactive audio editing and security platform built with Rust. While most audio tools focus either on professional editing or simple recording, EchoSafe bridges the gap by combining Adobe Audition-style spectral analysis with robust end-to-end encryption.
We chose this project to leverage Rust’s memory safety and performance in handling complex Digital Signal Processing (DSP) tasks. Our goal is to allow users to "see" sound frequencies and edit them with precision while ensuring that sensitive recordings remain private through integrated cryptographic layers.

Technical overview:

1. Basic Editing Engine

   Clip Management:
     Support for precise slicing, deleting, and merging of audio samples.

   Dynamic Gain Control:
     Volume adjustment for specific timestamps.
     Implementation of smooth Fade-in and Fade-out algorithms.
     Loudness Normalization: Balancing audio levels across the entire file.

   Time Stretching:
     Changing playback speed (faster/slower) without significantly distorting
     the original pitch.
3. Spectral Analysis & Advanced Processing
Inspired by professional tools like Adobe Audition, this module utilizes FFT(Fast Fourier Transform):

   Spectral Visualization:
     Viewing audio in the frequency domain to identify specific noises.

   Targeted Noise Reduction:
     Default background hum removal.
     Precise removal of "Pops" and "Clicks" by targeting specific frequency bands.
     User-defined frequency filtering (e.g., eliminating a high-pitched whistle).

   Voice Effects:
     Pitch shifting (changing voice height) and special audio filters.
5. Format Support & Interoperability

   Integration of the symphonia crate to support decoding of MP3, MP4, AAC, and ALAC.

   Standardized output to high-fidelity WAV format.
7. Security & Privacy Features

   End-to-End Encryption:
     Utilizing AES-256-GCM to lock processed audio files, making them unplayable without the correct key.

   Self-Destruct Mode:
     An optional "play-once" logic where the decrypted buffer and the source file are securely wiped from memory and disk immediately after playback.
Checkpoints

Checkpoint 1 (4/13 - 4/17)

1. Set up the project environment and integrate symphonia for multi-format decoding.

2. Implement basic volume scaling, clipping (slice/merge), and a functional CLI.

Checkpoint 2 (4/27 - 5/1)

1. Core Algorithm: Implement FFT-based spectral analysis and frequency-specific filtering.

2. Implement Time Stretching (speed control) and Pitch Shifting effects.

3. Develop Fade-in/out and audio balancing algorithms.

Final Submission (5/6)

1. Complete the AES-256 encryption/decryption module.

2. Finalize the "Self-Destruct" privacy logic.

3. Conduct performance optimization using rayon for parallel processing.

Current CLI Usage

1. Process audio into WAV:
   `cargo run -- process --input input.m4a --output processed.wav --normalize --fade 0.1`

2. Encrypt the processed file:
   `cargo run -- encrypt --input processed.wav --output processed.echo --password your-password`

3. Decrypt it back into a playable WAV:
   `cargo run -- decrypt --input processed.echo --output restored.wav --password your-password`

Possible Challenges

DSP Complexity: Implementing a clean Spectral Editing logic requires precise windowing and overlap-add methods in FFT to avoid audio artifacts.

Memory Management: Efficiently handling large audio buffers in memory while performing real-time encryption and playback.

References

RustFFT Library Documentation
https://docs.rs/rustfft/latest/rustfft/

Symphonia Audio Decoding Framework
https://github.com/pdeljanov/Symphonia
