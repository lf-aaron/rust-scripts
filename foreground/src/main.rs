use clap::Parser;
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use exr::prelude::*;
use webp;


struct ForegroundStruct {
    resolution: usize,
    ao: Vec<f32>,
    diffuse: Vec<f32>,
    glossy: Vec<f32>,
    index: Vec<f32>,
    matte: Vec<f32>,
}


enum ForegroundPass {
    AO,
    DIFFUSE,
    GLOSSY,
    INDEX,
    MATTE,
}


enum RGBAChannel {
    R,
    G,
    B,
    A,
}


fn channel_index(ch: RGBAChannel) -> usize {
    match ch {
        RGBAChannel::R => 0,
        RGBAChannel::G => 1,
        RGBAChannel::B => 2,
        RGBAChannel::A => 3,
    }
}


fn num_channels(pass: ForegroundPass) -> usize {
    match pass {
        ForegroundPass::AO => 3,
        ForegroundPass::DIFFUSE => 3,
        ForegroundPass::GLOSSY => 3,
        ForegroundPass::INDEX => 4,
        ForegroundPass::MATTE => 4,
    }
}


impl ForegroundStruct {
    fn new (resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            ao: vec![0_f32; n * num_channels(ForegroundPass::AO)],
            diffuse: vec![0_f32; n * num_channels(ForegroundPass::DIFFUSE)],
            glossy: vec![0_f32; n * num_channels(ForegroundPass::GLOSSY)],
            index: vec![0_f32; n * num_channels(ForegroundPass::INDEX)],
            matte: vec![0_f32; n * num_channels(ForegroundPass::MATTE)],
        }
    }

    fn update(ch: &mut Vec<f32>, channel_data: &Vec<f32>, num_channels: usize, channel_index: usize) {
        for (i, _) in channel_data.iter().enumerate() {
            ch[num_channels*i + channel_index] = channel_data[i];
        }
    }

    fn set_channel(&mut self, pass: ForegroundPass, channel: RGBAChannel, channel_data: &Vec<f32>) {
        let n = self.resolution * self.resolution;
        if channel_data.len() != n {
            panic!("Error: channel data has incorrect length ({:?})", channel_data.len());
        }

        match pass {
            ForegroundPass::AO => Self::update(&mut self.ao, channel_data, num_channels(pass), channel_index(channel)),
            ForegroundPass::DIFFUSE => Self::update(&mut self.diffuse, channel_data, num_channels(pass), channel_index(channel)),
            ForegroundPass::GLOSSY => Self::update(&mut self.glossy, channel_data, num_channels(pass), channel_index(channel)),
            ForegroundPass::INDEX => Self::update(&mut self.index, channel_data, num_channels(pass), channel_index(channel)),
            ForegroundPass::MATTE => Self::update(&mut self.matte, channel_data, num_channels(pass), channel_index(channel)),
        }
    }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    resolution: usize,

    #[clap(long)]
    config: String,

    #[clap(long, parse(from_os_str))]
    front: PathBuf,

    #[clap(long, parse(from_os_str))]
    rear: PathBuf,

    #[clap(long, parse(from_os_str))]
    upper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zfront: PathBuf,

    #[clap(long, parse(from_os_str))]
    zrear: PathBuf,

    #[clap(long, parse(from_os_str))]
    zupper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zplane: PathBuf,

    #[clap(long, parse(from_os_str))]
    light: PathBuf,

    #[clap(long, parse(from_os_str))]
    index: PathBuf,

    #[clap(long, parse(from_os_str))]
    matte: PathBuf,
}

// Used for picking channel from zmask
// Returns index of channel with largest value, or -1 if no values are above a threshold.
fn mask(x: &[u8]) -> i32 {
    let threshold = 5;
    if x[0] > threshold { return 0; }
    if x[1] > threshold { return 1; }
    if x[2] > threshold { return 2; }
    return -1;
}

fn get_asset_map() -> HashMap<[u8; 4], u8> {
    let array: [(u8, f32); 32] = [
        (0, 0.0),
        // (0, -1.1562982805507717e+33), // Mag shell
        (0, 46.93645477294922),       // NONE
        (1, -0.03498752787709236),
        (2, -7.442164937651292e-35),
        (3, -6.816108887753408e+29),
        (4, 0.00035458870115689933),
        (5, -2.1174496448267268e-37),
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
    ];

    HashMap::from(array.map(|v | { (v.1.to_be_bytes(), v.0) }))

}

// fn get_channels(header: &Header, layers: [&str; 4]) -> Vec<String>{
//     let mut sorted = Vec::<String>::new();
//     for layer in layers {
//         for channel in &header.channels.list {
//             let name = channel.name.to_string();
//             if name.as_str().contains(layer) {
//                 sorted.push(name);
//             }
//         }
//     }
//     return sorted;
// }


// Luma coefficients taken from Blender's OCIO config file
#[inline]
fn luminance(c: &[f32; 3]) -> f32 {
    return 0.2126*c[0] + 0.7152*c[1] + 0.0722*c[2];
}


// #[inline]
// fn rank_depth(z: &[f32; 4]) -> [u8; 4] {
//     // let ab: u8 = unsafe { mem::transmute(z[0] > z[1]) };
//     // let ac: u8 = unsafe { mem::transmute(z[0] > z[2]) };
//     // let ad: u8 = unsafe { mem::transmute(z[0] > z[3]) };
//     // let bc: u8 = unsafe { mem::transmute(z[1] > z[2]) };
//     // let bd: u8 = unsafe { mem::transmute(z[1] > z[3]) };
//     // let cd: u8 = unsafe { mem::transmute(z[2] > z[3]) };

//     let ab = z[0] > z[1];
//     let ac = z[0] > z[2];
//     let ad = z[0] > z[3];
//     let bc = z[1] > z[2];
//     let bd = z[1] > z[3];
//     let cd = z[2] > z[3];
    
//     let a = 0_u8;
//     let b = 0_u8;
//     let c = 0_u8;
//     let d = 0_u8;
// }


#[inline]
fn pack_index(x: &[u8; 4]) -> [u8; 3] {
    let a = x[0] | (x[1] & 0b11) << 6;
    let b = (x[1] & 0b111100) >> 2 | (x[2] & 0b1111) << 4;
    let c = (x[2] & 0b110000) >> 4 | x[3] << 2;
    return [a, b, c];
}


fn read_foreground_exr(path: &Path, resolution: usize) -> ForegroundStruct {

    // There are 21 channels in total.
    // Colors organized as (A, B, G, R), but some channels (AO, Diffuse, Glossy) do not contain alpha
    // 1) AO
    // 2) Combined
    // 3) Crypto00
    // 4) Crypto01
    // 5) Diffuse
    // 6) Glossy
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

    let mut obj = ForegroundStruct::new(resolution);

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };
    
    for (i, _) in channels.iter().enumerate() {
        match i {
            0 => obj.set_channel(ForegroundPass::AO, RGBAChannel::B, &f(&channels[i])),         // AO.B
            1 => obj.set_channel(ForegroundPass::AO, RGBAChannel::G, &f(&channels[i])),         // AO.G
            2 => obj.set_channel(ForegroundPass::AO, RGBAChannel::A, &f(&channels[i])),         // AO.R
            7 => obj.set_channel(ForegroundPass::INDEX, RGBAChannel::A, &f(&channels[i])),      // Crypto00.A
            8 => obj.set_channel(ForegroundPass::INDEX, RGBAChannel::B, &f(&channels[i])),      // Crypto00.B
            9 => obj.set_channel(ForegroundPass::INDEX, RGBAChannel::G, &f(&channels[i])),      // Crypto00.G
            10 => obj.set_channel(ForegroundPass::INDEX, RGBAChannel::R, &f(&channels[i])),     // Crypto00.R
            11 => obj.set_channel(ForegroundPass::MATTE, RGBAChannel::A, &f(&channels[i])),     // Crypto01.A
            12 => obj.set_channel(ForegroundPass::MATTE, RGBAChannel::B, &f(&channels[i])),     // Crypto01.B
            13 => obj.set_channel(ForegroundPass::MATTE, RGBAChannel::G, &f(&channels[i])),     // Crypto01.G
            14 => obj.set_channel(ForegroundPass::MATTE, RGBAChannel::R, &f(&channels[i])),     // Crypto01.R
            15 => obj.set_channel(ForegroundPass::DIFFUSE, RGBAChannel::B, &f(&channels[i])),   // Diffuse.B
            16 => obj.set_channel(ForegroundPass::DIFFUSE, RGBAChannel::G, &f(&channels[i])),   // Diffuse.G
            17 => obj.set_channel(ForegroundPass::DIFFUSE, RGBAChannel::R, &f(&channels[i])),   // Diffuse.R
            18 => obj.set_channel(ForegroundPass::GLOSSY, RGBAChannel::B, &f(&channels[i])),    // Glossy.B
            19 => obj.set_channel(ForegroundPass::GLOSSY, RGBAChannel::G, &f(&channels[i])),    // Glossy.G
            20 => obj.set_channel(ForegroundPass::GLOSSY, RGBAChannel::R, &f(&channels[i])),    // Glossy.R
            _ => {},
        };
    }

    return obj;
}


fn read_depth_exr(path: &Path) -> Vec<f32> {
    
    let channel = exr::prelude::read()
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
        
    let pixels = match &channel.last().unwrap().sample_data {
        exr::prelude::FlatSamples::F32(x) => x,
        _ => panic!("Unexpected channel type"),
    };
    
    return pixels.to_owned();
}


fn composite(
    size: usize,
    map: &HashMap<[u8; 4], u8>,
    front: &ForegroundStruct,
    rear: &ForegroundStruct,
    upper: &ForegroundStruct,
    zfront: &Vec<f32>,
    zrear: &Vec<f32>,
    zupper: &Vec<f32>,
    zplane: &Vec<f32>
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let num_pixels = size*size;
    let mut light = vec![0_u8; num_pixels * 3];
    let mut index = vec![0_u8; num_pixels * 3];
    let mut matte = vec![0_u8; num_pixels * 3];

    for i in 0..num_pixels {

        // Sort Z

        // Map index

        // Add matte
        
        // Combine light passes

        // Convert to u8

        // let zfront = &zfront[3*i..3*i+3];
        // let zrear = &zrear[3*i..3*i+3];
        // let zupper = &zupper[3*i..3*i+3];

        // let idx1 = &front[8*i..8*i+4];
        // let idx2 = &rear[8*i..8*i+4];
        // let idx3 = &upper[8*i..8*i+4];

        // let val1 = &front[8*i+4..8*i+8];
        // let val2 = &rear[8*i+4..8*i+8];
        // let val3 = &upper[8*i+4..8*i+8];

        // let (idx, val) = match mask(z) {
        //     0 => (idx1, val1),
        //     1 => (idx2, val2),
        //     2 => (idx3, val3),
        //     _ => ([0_f32; 4].as_slice(), [0_f32; 4].as_slice())
        // };

        // // let id = idx.into_iter().map(|x| { map[&x.to_be_bytes()] }).collect::<Vec<u8>>();
        // let id = idx.into_iter().map(|x| {
        //     let y = match map.get(&x.to_be_bytes()) {
        //         None => panic!("Missing key: {}", *x),
        //         Some(num) => *num
        //     };
        //     return y;
        // }).collect::<Vec<u8>>();
        // let i_r = id[0] | (id[1] & 0b11) << 6;
        // let i_g = (id[1] & 0b111100) >> 2 | (id[2] & 0b1111) << 4;
        // let i_b = (id[2] & 0b110000) >> 4 | id[3] << 2;

        // let m_r = (2.0 * val[1] * 255_f32) as u8;
        // let m_g = (2.0 * val[2] * 255_f32) as u8;
        // let m_b = (2.0 * val[3] * 255_f32) as u8;

        // index.splice(3*i..3*i+3, [i_r, i_g, i_b]);
        // matte.splice(3*i..3*i+3, [m_r, m_g, m_b]);
    }
    return (light, index, matte);
}


fn save_webp(path: PathBuf, size: usize, pixels: &Vec<u8>) {
    // print!("path: {:?}\n", path);
    let img = webp::Encoder::from_rgb(pixels, size as u32, size as u32).encode_lossless();
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}


fn main() {
    let args = CliArgs::parse();
    
    let size = args.resolution;
    let config = &args.config;
    let front_dir = &args.front;
    let rear_dir = &args.rear;
    let upper_dir = &args.upper;
    let zfront_dir = &args.zfront;
    let zrear_dir = &args.zrear;
    let zupper_dir = &args.zupper;
    let zplane_path = &args.zplane;
    let light_dir = &args.light;
    let index_dir = &args.index;
    let matte_dir = &args.matte;

    let front_files = fs::read_dir(front_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let rear_files = fs::read_dir(rear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let upper_files = fs::read_dir(upper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zfront_files = fs::read_dir(zfront_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zrear_files = fs::read_dir(zrear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zupper_files = fs::read_dir(zupper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if front_files.len() != num_frames { panic!("Missing 'Front' files"); }
    if rear_files.len() != num_frames { panic!("Missing 'Rear' files"); }
    if upper_files.len() != num_frames { panic!("Missing 'Upper' files"); }
    if zfront_files.len() != num_frames { panic!("Missing 'Z Front' files"); }
    if zrear_files.len() != num_frames { panic!("Missing 'Z Rear' files"); }
    if zupper_files.len() != num_frames { panic!("Missing 'Z Upper' files"); }

    let zplane = read_depth_exr(&zplane_path);

    let map = get_asset_map();

    for i in 0..num_frames {
        let f_front = &front_files[i];
        let f_rear = &rear_files[i];
        let f_upper = &upper_files[i];
        let f_zfront = &zfront_files[i];
        let f_zrear = &zrear_files[i];
        let f_zupper = &zupper_files[i];

        let front = read_foreground_exr(&f_front.path(), size);
        let rear = read_foreground_exr(&f_rear.path(), size);
        let upper = read_foreground_exr(&f_upper.path(), size);
        let zfront = read_depth_exr(&f_zfront.path());
        let zrear = read_depth_exr(&f_zrear.path());
        let zupper = read_depth_exr(&f_zupper.path());

        let (light, index, matte) = composite(
            size,
            &map,
            &front,
            &rear,
            &upper,
            &zfront,
            &zrear,
            &zupper,
            &zplane,
        );

        save_webp(light_dir.join(config), size, &light);
        save_webp(index_dir.join(config), size, &index);
        save_webp(matte_dir.join(config), size, &matte);
    }
}
