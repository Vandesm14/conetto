use crate::Tts;
use async_trait::async_trait;
use core::fmt;
use hound::WavSpec;
use lowpass_filter::lowpass_filter;
use spellabet::{PhoneticConverter, SpellingAlphabet};
use std::{
  fmt::{Display, Formatter},
  rc::Rc,
};

#[async_trait]
pub trait Render {
  async fn render(&self, samples: &mut Vec<f32>, tts: &mut Tts);
}

pub enum Encoding {
  Words,
  Ascii,
  Phonetic,
}

pub struct Speak<'a> {
  pub text: &'a str,

  /// If None, a random voice will be selected
  pub voice: Option<VoiceModel>,

  /// If None, the no encoding will be used
  pub encoding: Option<Encoding>,
}

impl<'a> Speak<'a> {
  pub fn new(text: &'a str) -> Self {
    Self {
      encoding: None,
      text,
      voice: None,
    }
  }

  pub fn with_voice(mut self, voice: VoiceModel) -> Self {
    self.voice = Some(voice);
    self
  }

  pub fn with_encoding(mut self, encoding: Encoding) -> Self {
    self.encoding = Some(encoding);
    self
  }

  fn model(&self) -> Option<String> {
    let voice_model = self.voice;
    voice_model?;

    Some(voice_model.unwrap().to_string())
  }
}

#[async_trait]
impl<'a> Render for Speak<'a> {
  async fn render(&self, samples: &mut Vec<f32>, tts: &mut Tts) {
    match self.encoding {
      Some(Encoding::Words) => {
        // Split the secret phrase into words
        let words = self.text.split_whitespace();

        // Run through each word and TTS samples
        for word in words {
          let more_samples = tts.generate(word, self.model()).await;
          samples.extend(more_samples);
        }
      }
      Some(Encoding::Ascii) => {
        // Convert secret phrase into ascii codes (String of numbers)
        let words = self
          .text
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
            let more_samples =
              tts.generate(&char.to_string(), self.model()).await;
            samples.extend(more_samples);

            // Short pause between letters
            Pause(600).render(samples, tts).await;
          }

          // Long pause between words
          Pause(400).render(samples, tts).await;
        }
      }
      Some(Encoding::Phonetic) => {
        {
          let converter = PhoneticConverter::new(&SpellingAlphabet::Nato);

          // Convert secret phrase into phonetic alphabet
          let string = converter.convert(self.text);

          // Split into words
          let words = string.split_whitespace();

          // Run throuch each word and TTS samples
          for word in words {
            if word.to_lowercase().as_str() == "space" {
              // Long pause between words
              Pause(600).render(samples, tts).await;
              continue;
            }

            let more_samples = tts.generate(word, self.model()).await;
            samples.extend(more_samples);

            // Short pause between words
            Pause(160).render(samples, tts).await;
          }
        }
      }
      None => {
        let more_samples = tts.generate(self.text, self.model()).await;
        samples.extend(more_samples);
      }
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum VoiceModel {
  A,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
}

impl Display for VoiceModel {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      VoiceModel::A => write!(f, "A"),
      VoiceModel::B => write!(f, "B"),
      VoiceModel::C => write!(f, "C"),
      VoiceModel::D => write!(f, "D"),
      VoiceModel::E => write!(f, "E"),
      VoiceModel::F => write!(f, "F"),
      VoiceModel::G => write!(f, "G"),
      VoiceModel::H => write!(f, "H"),
      VoiceModel::I => write!(f, "I"),
      VoiceModel::J => write!(f, "J"),
    }
  }
}

pub struct Pause(pub u32);

#[async_trait]
impl Render for Pause {
  async fn render(&self, samples: &mut Vec<f32>, _tts: &mut Tts) {
    samples.extend(vec![0.0f32; 24 * (self.0 as usize)])
  }
}

/// Renders all clips and returns the WAV samples
pub async fn render_all(
  clips: impl Iterator<Item = Rc<dyn Render>>,
) -> Vec<f32> {
  let mut tts = Tts::new();
  let mut samples = vec![];

  for clip in clips {
    clip.render(&mut samples, &mut tts).await;
  }

  samples
}

pub fn save_audio_file(samples: &mut [f32], path: &str) {
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

  let mut writer = hound::WavWriter::create(path, output_spec).unwrap();

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