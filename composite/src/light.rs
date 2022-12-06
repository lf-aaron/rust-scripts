use exr::prelude::*;
use std::path::{Path};
use util::{RGBAChannel};

pub struct EXRData {
    resolution: usize,
    pub ao: Vec<f32>,
    pub diffuse: Vec<f32>,
    pub glossy: Vec<f32>,
}

pub enum RenderPass {
    AO,
    DIFFUSE,
    GLOSSY,
}

impl EXRData {
    fn new(resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            ao: vec![0_f32; n * 3],
            diffuse: vec![0_f32; n * 3],
            glossy: vec![0_f32; n * 3],
        }
    }

    fn set_channel(&mut self, channel_data: Vec<f32>, pass: RenderPass, channel: RGBAChannel) {
        let n = self.resolution * self.resolution;
        if channel_data.len() != n {
            panic!(
                "Error: channel data has incorrect length ({:?})",
                channel_data.len()
            );
        }

        let offset = n * match channel {
            RGBAChannel::R => 0,
            RGBAChannel::G => 1,
            RGBAChannel::B => 2,
            RGBAChannel::A => 3,
        };

        match pass {
            RenderPass::AO => {
                self.ao.splice(offset..offset + n, channel_data);
            }
            RenderPass::DIFFUSE => {
                self.diffuse.splice(offset..offset + n, channel_data);
            }
            RenderPass::GLOSSY => {
                self.glossy.splice(offset..offset + n, channel_data);
            }
        };
    }
}

pub fn read_exr(path: &Path, resolution: u32) -> EXRData {
    // New files should have 13 channels in total.
    // Colors organized as (A, B, G, R), but some channels (AO, Diffuse, Glossy) do not contain alpha
    // 1) AO
    // 2) Combined
    // 3) Diffuse
    // 4) Glossy
    let channels = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .first_valid_layer()
        .all_attributes()
        .from_file(path)
        .unwrap()
        .layer_data
        .channel_data
        .list;

    let mut obj = EXRData::new(resolution as usize);

    let f = |ch: &AnyChannel<FlatSamples>| match &ch.sample_data {
        exr::prelude::FlatSamples::F32(x) => x.to_owned(),
        _ => panic!("Unexpected channel type"),
    };

    for (i, _) in channels.iter().enumerate() {
        match i {
            0 => obj.set_channel(f(&channels[i]), RenderPass::AO, RGBAChannel::B), // AO.B
            1 => obj.set_channel(f(&channels[i]), RenderPass::AO, RGBAChannel::G), // AO.G
            2 => obj.set_channel(f(&channels[i]), RenderPass::AO, RGBAChannel::R), // AO.R
            // 3                                                                              // Combined.A
            // 4                                                                              // Combined.B
            // 5                                                                              // Combined.G
            // 6                                                                              // Combined.R
            7 => obj.set_channel(f(&channels[i]), RenderPass::DIFFUSE, RGBAChannel::B), // Diffuse.B
            8 => obj.set_channel(f(&channels[i]), RenderPass::DIFFUSE, RGBAChannel::G), // Diffuse.G
            9 => obj.set_channel(f(&channels[i]), RenderPass::DIFFUSE, RGBAChannel::R), // Diffuse.R
            10 => obj.set_channel(f(&channels[i]), RenderPass::GLOSSY, RGBAChannel::B), // Glossy.B
            11 => obj.set_channel(f(&channels[i]), RenderPass::GLOSSY, RGBAChannel::G), // Glossy.G
            12 => obj.set_channel(f(&channels[i]), RenderPass::GLOSSY, RGBAChannel::R), // Glossy.R
            _ => {}
        };
    }

    return obj;
}
