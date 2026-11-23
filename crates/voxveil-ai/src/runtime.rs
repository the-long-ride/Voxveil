use crate::{Model, spectral::Spectral};
use ort::{session::Session, value::Tensor};
use std::path::Path;
use voxveil_model_api::SeparationError;

pub(crate) struct OnnxModel {
    model: Model,
    sessions: Vec<Session>,
    spectral: Spectral,
}

impl OnnxModel {
    pub fn load(model: Model, directory: &Path, library: &Path) -> Result<Self, SeparationError> {
        if !library.is_absolute() || !library.is_file() {
            return Err(SeparationError::ModelUnavailable);
        }
        ort::init_from(library)
            .map_err(|_| SeparationError::ModelUnavailable)?
            .commit();
        let (files, spectral) = match model {
            Model::Spleeter => (
                vec!["vocals.fp16.onnx", "accompaniment.fp16.onnx"],
                Spectral::new(4096, 512, 1024, false),
            ),
            Model::MdxInstHq3 => (
                vec!["mdx-inst-hq3.onnx"],
                Spectral::new(6144, 256, 3072, true),
            ),
        };
        let mut sessions = Vec::new();
        for file in files {
            sessions.push(
                Session::builder()
                    .and_then(|builder| builder.with_intra_threads(4))
                    .and_then(|builder| builder.with_inter_threads(1))
                    .and_then(|builder| builder.commit_from_file(directory.join(file)))
                    .map_err(|_| SeparationError::ModelUnavailable)?,
            );
        }
        Ok(Self {
            model,
            sessions,
            spectral,
        })
    }

    pub fn separate(&mut self, input: &[f32]) -> Result<Vec<f32>, SeparationError> {
        let mut spectra = self.spectral.analyze(input);
        let width = self.spectral.n / 2 + 1;
        let frames = self.spectral.frames;
        let bins = self.spectral.bins;
        match self.model {
            Model::Spleeter => {
                let mut magnitude = Vec::with_capacity(2 * frames * bins);
                for channel in 0..2 {
                    for frame in 0..frames {
                        let start = (channel * frames + frame) * width;
                        magnitude.extend(spectra[start..start + bins].iter().map(|z| z.norm()));
                    }
                }
                let vocals = infer(
                    &mut self.sessions[0],
                    &[2, 1, frames, bins],
                    magnitude.clone(),
                )?;
                let accompaniment = infer(&mut self.sessions[1], &[2, 1, frames, bins], magnitude)?;
                for channel in 0..2 {
                    for frame in 0..frames {
                        for bin in 0..width {
                            let z = &mut spectra[(channel * frames + frame) * width + bin];
                            if bin < bins {
                                let i = (channel * frames + frame) * bins + bin;
                                let a = accompaniment[i].powi(2);
                                let v = vocals[i].powi(2);
                                *z *= (a + 5e-11) / (a + v + 1e-10);
                            } else {
                                *z = Default::default();
                            }
                        }
                    }
                }
            }
            Model::MdxInstHq3 => {
                let mut tensor = vec![0.0; 4 * bins * frames];
                for channel in 0..2 {
                    for bin in 3..bins {
                        for frame in 0..frames {
                            let z = spectra[(channel * frames + frame) * width + bin];
                            tensor[(channel * 2 * bins + bin) * frames + frame] = z.re;
                            tensor[((channel * 2 + 1) * bins + bin) * frames + frame] = z.im;
                        }
                    }
                }
                let prediction = infer(&mut self.sessions[0], &[1, 4, bins, frames], tensor)?;
                spectra.fill(Default::default());
                for channel in 0..2 {
                    for bin in 0..bins {
                        for frame in 0..frames {
                            let z = &mut spectra[(channel * frames + frame) * width + bin];
                            z.re = prediction[(channel * 2 * bins + bin) * frames + frame] * 1.022;
                            z.im = prediction[((channel * 2 + 1) * bins + bin) * frames + frame]
                                * 1.022;
                        }
                    }
                }
            }
        }
        Ok(self.spectral.synthesize(&spectra, input.len() / 2))
    }
}

fn infer(
    session: &mut Session,
    shape: &[usize],
    data: Vec<f32>,
) -> Result<Vec<f32>, SeparationError> {
    let input =
        Tensor::from_array((shape.to_vec(), data)).map_err(|_| SeparationError::BackendFailure)?;
    let outputs = session
        .run(ort::inputs![input])
        .map_err(|_| SeparationError::BackendFailure)?;
    let (actual_shape, samples) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|_| SeparationError::BackendFailure)?;
    if actual_shape.iter().map(|&v| v as usize).collect::<Vec<_>>() != shape
        || samples.iter().any(|x| !x.is_finite())
    {
        return Err(SeparationError::BackendFailure);
    }
    Ok(samples.to_vec())
}
