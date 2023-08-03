#![feature(iter_intersperse)]

use std::time::Instant;

use conet::Tts;
use hound::WavSpec;
use lowpass_filter::lowpass_filter;
use spellabet::{PhoneticConverter, SpellingAlphabet};

#[tokio::main]
async fn main() {
  let start_time = Instant::now();
  let mut tts = Tts::new();
  let secret_phrase = "There is nothing left to fear but fear itself.";

  // Create initial preamble
  let mut samples = tts
    .generate(
      "This is an automated broadcast. Please listen carefully.",
      Some("en-US-Standard-F"),
    )
    .await;

  // Long pause between preamble and secret phrase
  samples.extend([0.0f32; 24_000]);

  ascii_encoding(secret_phrase, &mut samples, &mut tts).await;

  let end_time = Instant::now();

  println!(
    "Generated {} samples in {}ms",
    samples.len(),
    end_time.duration_since(start_time).as_millis().to_string()
  );

  // Save audio file
  save_audio_file(&mut samples);
}

async fn ascii_encoding(string: &str, samples: &mut Vec<f32>, tts: &mut Tts) {
  // Convert secret phrase into ascii codes (String of numbers)
  let words = string
    .as_bytes()
    .iter()
    // Convert each byte into a string, padded with 0s
    .map(|b| format!("{:0>3}", b))
    .reduce(|a, b| a + &b)
    .unwrap();

  // Split the ascii string into chars
  let words = words.chars().collect::<Vec<_>>();

  // Split into chunks of 5
  let words = words.chunks(5);

  // Run throuch each chunk and TTS samples
  for word in words {
    for char in word {
      let more_samples = tts.generate(&char.to_string(), None).await;
      samples.extend(more_samples);

      // Short pause between letters
      samples.extend([0.0f32; 4_000]);
    }

    // Long pause between words
    samples.extend([0.0f32; 10_000]);
  }
}

async fn phonetic_encoding(
  string: &str,
  samples: &mut Vec<f32>,
  tts: &mut Tts,
) {
  let converter = PhoneticConverter::new(&SpellingAlphabet::Nato);

  // Convert secret phrase into phonetic alphabet
  let string = converter.convert(string);

  // Split into words
  let words = string.split_whitespace();

  // Run throuch each word and TTS samples
  for word in words {
    let more_samples = tts.generate(word, None).await;
    samples.extend(more_samples);

    // Short pause between words
    samples.extend([0.0f32; 4_000]);
  }
}

fn save_audio_file(samples: &mut [f32]) {
  let spec = WavSpec {
    channels: 1,
    sample_rate: 24_000,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
  };

  let output_spec = WavSpec {
    channels: 1,
    sample_rate: 8_000,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
  };

  let mut writer =
    hound::WavWriter::create("/tmp/conet/audio.wav", output_spec).unwrap();

  lowpass_filter(samples, 24_000.0, 8_000.0);

  let downsampling_factor =
    (spec.sample_rate / output_spec.sample_rate) as usize;

  // Downsample to 8kHz
  for sample in samples
    .iter()
    .skip(downsampling_factor - 1)
    .step_by(downsampling_factor)
    .copied()
  {
    writer
      // .write_sample((sample as i32).clamp(0, 2i32.pow(10)) << 3)
      .write_sample(sample as i32)
      .unwrap();
  }
}
