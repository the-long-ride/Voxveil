use rubato::{FftFixedIn, Resampler};
use voxveil_model_api::SeparationError;

pub(crate) fn convert(
    input: &[f32],
    from: usize,
    to: usize,
    frames: usize,
) -> Result<Vec<f32>, SeparationError> {
    if from == to {
        let mut output = input.to_vec();
        output.resize(frames * 2, 0.0);
        output.truncate(frames * 2);
        return Ok(output);
    }
    let mut resampler = FftFixedIn::<f32>::new(from, to, 1024, 2, 2)
        .map_err(|_| SeparationError::BackendFailure)?;
    let delay = resampler.output_delay();
    let mut collected = [Vec::new(), Vec::new()];
    let mut cursor = 0;
    while collected[0].len() < frames + delay {
        let count = resampler.input_frames_next();
        let mut block = vec![vec![0.0; count]; 2];
        for i in 0..count {
            for channel in 0..2 {
                block[channel][i] = input
                    .get((cursor + i) * 2 + channel)
                    .copied()
                    .unwrap_or(0.0);
            }
        }
        cursor += count;
        let output = resampler
            .process(&block, None)
            .map_err(|_| SeparationError::BackendFailure)?;
        for channel in 0..2 {
            collected[channel].extend_from_slice(&output[channel]);
        }
    }
    Ok((delay..delay + frames)
        .flat_map(|i| [collected[0][i], collected[1][i]])
        .collect())
}
