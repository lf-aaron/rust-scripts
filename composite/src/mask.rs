use exr::prelude::*;
use std::mem::{transmute};
use std::path::{Path};
use util::{RGBAChannel};

pub struct EXRData {
    resolution: usize,
    pub depth: Vec<f32>,
    pub index: Vec<u32>,
    pub matte: Vec<f32>,
}

pub enum RenderPass {
    DEPTH,
    INDEX,
    MATTE,
}

impl EXRData {
    fn new(resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            depth: vec![0_f32; n],
            index: vec![0_u32; n * 4],
            matte: vec![0_f32; n * 4],
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
            RenderPass::DEPTH => {
                self.matte.splice(offset..offset + n, channel_data);
            }
            RenderPass::INDEX => {
                self.index.splice(offset..offset + n, unsafe {
                    channel_data
                        .into_iter()
                        .map(|x| transmute::<f32, u32>(x))
                        .collect::<Vec<u32>>()
                });
            }
            RenderPass::MATTE => {
                self.matte.splice(offset..offset + n, channel_data);
            }
        };
    }
}

pub fn read_exr(path: &Path, resolution: u32) -> EXRData {
    // There are 13 channels in total.
    // Colors organized as (A, B, G, R)
    // 1) 4x Combined
    // 2) 4x Crypto00
    // 3) 4x Crypto01
    // 4) 1x Depth
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
            // 0                                                                        // Combined.A
            // 1                                                                        // Combined.B
            // 2                                                                        // Combined.G
            // 3                                                                        // Combined.R
            4 => obj.set_channel(f(&channels[i]), RenderPass::MATTE, RGBAChannel::G), // Crypto00.A
            5 => obj.set_channel(f(&channels[i]), RenderPass::INDEX, RGBAChannel::G), // Crypto00.B
            6 => obj.set_channel(f(&channels[i]), RenderPass::MATTE, RGBAChannel::R), // Crypto00.G
            7 => obj.set_channel(f(&channels[i]), RenderPass::INDEX, RGBAChannel::R), // Crypto00.R
            8 => obj.set_channel(f(&channels[i]), RenderPass::MATTE, RGBAChannel::A), // Crypto01.A
            9 => obj.set_channel(f(&channels[i]), RenderPass::INDEX, RGBAChannel::A), // Crypto01.B
            10 => obj.set_channel(f(&channels[i]), RenderPass::MATTE, RGBAChannel::B), // Crypto01.G
            11 => obj.set_channel(f(&channels[i]), RenderPass::INDEX, RGBAChannel::B), // Crypto01.R
            12 => obj.set_channel(f(&channels[i]), RenderPass::DEPTH, RGBAChannel::R), // Depth
            _ => {}
        };
    }

    return obj;
}

pub static index_map: [(u32, f32); 40] = [
  (0, 0.0), // NONE
  (1, 46.93645477294922), // VOID
  (2, -0.03498752787709236),
  (3, -7.442164937651292e-35),
  (4, -6.816108887753408e+29),
  (5, 0.00035458870115689933),
  (6, 1.4020313126302311e+32),
  (7, -1.0356748461253123e-29),
  (8, -2.9085143335341026e+36),
  (9, 1.3880169547064725e-07),
  (10, -1.259480075076364e+31),
  (11, 9.950111644430328e-20),
  (12, 7.755555963998422e+23),
  (13, 7.694573644696632e-19),
  (14, -5.1650727722774545e-23),
  (15, 9.80960464477539),
  (16, -2.863075394543557e-07),
  (17, -1.1106028290273499e+26),
  (18, 5.081761389253177e+22),
  (19, -6.4202393950590105e+25),
  (20, -4.099688753251169e+19),
  (21, -4.738090716008833e+34),
  (22, 1.3174184410047474e-08),
  (23, -0.014175964519381523),
  (24, 2.4984514311654493e-05),
  (25, -8.232201253122184e-06),
  (26, 1.2103820479584479e-20),
  (27, -2.508242528606597e-12),
  (28, 1.5731503249895985e+26),
  (29, 1.4262572893553038e-11),
  (30, -84473296.0),
  (31, -6.486369792231469e-37),
  (32, 1.150444436850863e+20),
  (33, -1.180638517484824e-38),
  (34, 3.6098729115699803e-36),
  (35, -1.0834222605653176e-29),
  (36, -1.1292286235016067e-29),
  (37, 2.9290276870597154e-09),
  (38, 2.641427494857044e-31),
  (39, 2.353400999332558e-15),
];
