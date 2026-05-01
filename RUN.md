# RUN.md

## Prerequisites

- Git
- Rust and Cargo

You can verify Rust is installed with:

```bash
rustc --version
cargo --version
```

## Steps To Run

1. Clone the repository:

```bash
git clone https://github.com/xinyaogu925/CS128H-Final-Project.git
```

2. Enter the project directory:

```bash
cd CS128H-Final-Project
```

3. Process the sample audio file into a WAV file:

```bash
cargo run -- process --input input.m4a --output processed.wav --normalize --fade 0.1
```

4. Encrypt the processed WAV file:

```bash
cargo run -- encrypt --input processed.wav --output processed.echo --password demo-password
```

5. Decrypt the encrypted file back into a playable WAV file:

```bash
cargo run -- decrypt --input processed.echo --output restored.wav --password demo-password
```

## Expected Output

After the commands above finish, you should see these files in the project directory:

- `processed.wav`
- `processed.echo`
- `restored.wav`

## Optional Check

To run the unit tests:

```bash
cargo test
```
