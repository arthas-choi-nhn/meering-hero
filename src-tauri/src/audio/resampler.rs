/// Resamples audio from source sample rate to 16kHz mono f32.
pub struct Resampler {
    source_rate: u32,
    source_channels: u16,
}

impl Resampler {
    pub fn new(source_rate: u32, source_channels: u16) -> Self {
        Self {
            source_rate,
            source_channels,
        }
    }

    /// Resample interleaved f32 audio to 16kHz mono f32.
    /// Uses linear interpolation for simplicity.
    pub fn resample(&self, input: &[f32]) -> Vec<f32> {
        // Step 1: Convert to mono by averaging channels
        let mono = if self.source_channels > 1 {
            let ch = self.source_channels as usize;
            input
                .chunks_exact(ch)
                .map(|frame| frame.iter().sum::<f32>() / ch as f32)
                .collect::<Vec<_>>()
        } else {
            input.to_vec()
        };

        // Step 2: Resample to 16kHz using linear interpolation
        let target_rate: u32 = 16000;
        if self.source_rate == target_rate {
            return mono;
        }

        let ratio = self.source_rate as f64 / target_rate as f64;
        let output_len = (mono.len() as f64 / ratio).ceil() as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_pos = i as f64 * ratio;
            let src_idx = src_pos as usize;
            let frac = src_pos - src_idx as f64;

            let sample = if src_idx + 1 < mono.len() {
                mono[src_idx] as f64 * (1.0 - frac) + mono[src_idx + 1] as f64 * frac
            } else if src_idx < mono.len() {
                mono[src_idx] as f64
            } else {
                0.0
            };

            output.push(sample as f32);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mono_passthrough() {
        let resampler = Resampler::new(16000, 1);
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let output = resampler.resample(&input);
        assert_eq!(output.len(), input.len());
        for (a, b) in output.iter().zip(input.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_stereo_to_mono() {
        let resampler = Resampler::new(16000, 2);
        // stereo: L=0.2, R=0.4 → mono = 0.3
        let input = vec![0.2, 0.4, 0.6, 0.8];
        let output = resampler.resample(&input);
        assert_eq!(output.len(), 2);
        assert!((output[0] - 0.3).abs() < 1e-6);
        assert!((output[1] - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_downsample_48k_to_16k() {
        let resampler = Resampler::new(48000, 1);
        // 48 samples at 48kHz = 1ms → should produce ~16 samples at 16kHz
        let input: Vec<f32> = (0..480).map(|i| (i as f32 / 480.0)).collect();
        let output = resampler.resample(&input);
        // 480 samples at 48kHz → 160 samples at 16kHz
        assert_eq!(output.len(), 160);
    }
}
