use arrayfire::*;
use clap::Parser;
use image::{EncodableLayout};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::fs;
use std::mem::{transmute};
use std::ops::{Not, Shl, Shr, BitAnd, BitOr};
use std::path::{Path, PathBuf};
use exr::prelude::*;
use webp;


struct ForegroundStruct {
    resolution: usize,
    ao: Vec<f32>,
    diffuse: Vec<f32>,
    glossy: Vec<f32>,
    index: Vec<u32>,
    matte: Vec<f32>,
}

#[derive(Debug)]
enum ForegroundPass {
    AO,
    DIFFUSE,
    GLOSSY,
    INDEX,
    MATTE,
}

#[derive(Debug)]
enum RGBAChannel {
    R,
    G,
    B,
    A,
}

#[derive(Debug)]
enum WebpCompressionType {
    LOSSY(f32),
    LOSSLESS,
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
            index: vec![0_u32; n * num_channels(ForegroundPass::INDEX)],
            matte: vec![0_f32; n * num_channels(ForegroundPass::MATTE)],
        }
    }

    
    fn set_channel(&mut self, channel_data: Vec<f32>, pass: ForegroundPass, channel: RGBAChannel) {
        
        let n = self.resolution * self.resolution;
        if channel_data.len() != n {
            panic!("Error: channel data has incorrect length ({:?})", channel_data.len());
        }

        let offset = n * match channel {
            RGBAChannel::R => 0,
            RGBAChannel::G => 1,
            RGBAChannel::B => 2,
            RGBAChannel::A => 3,
        };

        match pass {
            ForegroundPass::AO => { self.ao.splice(offset..offset+n, channel_data); },
            ForegroundPass::DIFFUSE => { self.diffuse.splice(offset..offset+n, channel_data); },
            ForegroundPass::GLOSSY => { self.glossy.splice(offset..offset+n, channel_data); },
            ForegroundPass::INDEX => { self.index.splice(offset..offset+n, unsafe { transmute::<Vec<f32>, Vec<u32>>(channel_data) }); },
            ForegroundPass::MATTE => { self.matte.splice(offset..offset+n, channel_data); },
        };
    }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    resolution: u32,

    #[clap(long, parse(from_os_str))]
    front: PathBuf,

    #[clap(long, parse(from_os_str))]
    rear: PathBuf,

    #[clap(long, parse(from_os_str))]
    upper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

    #[clap(long, parse(from_os_str))]
    light: PathBuf,

    #[clap(long, parse(from_os_str))]
    index: PathBuf,

    #[clap(long, parse(from_os_str))]
    matte: PathBuf,
}


fn get_asset_map() -> HashMap<u32, u8> {
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

    HashMap::from(array.map(|v | { ( unsafe { transmute::<f32, u32>(v.1) }, v.0) }))

}


// #[inline]
// fn pack_index(x: &[u8; 4]) -> [u8; 3] {
//     let a = x[0] | (x[1] & 0b11) << 6;
//     let b = (x[1] & 0b111100) >> 2 | (x[2] & 0b1111) << 4;
//     let c = (x[2] & 0b110000) >> 4 | x[3] << 2;
//     return [a, b, c];
// }


fn read_foreground_exr(path: &Path, resolution: u32) -> ForegroundStruct {

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

    let mut obj = ForegroundStruct::new(resolution as usize);

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };
    
    for (i, _) in channels.iter().enumerate() {
        match i {
            0 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::B),         // AO.B
            1 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::G),         // AO.G
            2 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::R),         // AO.R
            7 => obj.set_channel(f(&channels[i]), ForegroundPass::INDEX, RGBAChannel::A),      // Crypto00.A
            8 => obj.set_channel(f(&channels[i]), ForegroundPass::INDEX, RGBAChannel::B),      // Crypto00.B
            9 => obj.set_channel(f(&channels[i]), ForegroundPass::INDEX, RGBAChannel::G),      // Crypto00.G
            10 => obj.set_channel(f(&channels[i]), ForegroundPass::INDEX, RGBAChannel::R),     // Crypto00.R
            11 => obj.set_channel(f(&channels[i]), ForegroundPass::MATTE, RGBAChannel::A),     // Crypto01.A
            12 => obj.set_channel(f(&channels[i]), ForegroundPass::MATTE, RGBAChannel::B),     // Crypto01.B
            13 => obj.set_channel(f(&channels[i]), ForegroundPass::MATTE, RGBAChannel::G),     // Crypto01.G
            14 => obj.set_channel(f(&channels[i]), ForegroundPass::MATTE, RGBAChannel::R),     // Crypto01.R
            15 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::B),   // Diffuse.B
            16 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::G),   // Diffuse.G
            17 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::R),   // Diffuse.R
            18 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::B),    // Glossy.B
            19 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::G),    // Glossy.G
            20 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::R),    // Glossy.R
            _ => {},
        };
    }

    return obj;
}


fn composite(
    map: &HashMap<u32, u8>,
    front: ForegroundStruct,
    rear: ForegroundStruct,
    upper: ForegroundStruct,
    zmask: &Vec<u8>,
    size: u64,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    
    let dim3 = dim4!(size, size, 3);
    let dim4 = dim4!(size, size, 4);

    let mut light = vec!(0; dim3.elements() as usize);
    let mut index = vec!(0; dim3.elements() as usize);
    let mut matte = vec!(0; dim3.elements() as usize);

    let mut a_zmask = Array::new(zmask, dim4!(3, size, size)).cast::<bool>();
    a_zmask = reorder_v2(&a_zmask, 1, 2, Some(vec![0]));

    // NOTE: `1:1:0` means all elements along axis
    let m_front = view!(a_zmask[1:1:0, 1:1:0, 0:0:1]);
    let m_rear = view!(a_zmask[1:1:0, 1:1:0, 1:1:1]);
    let m_upper = view!(a_zmask[1:1:0, 1:1:0, 2:2:1]);

    // **************************************************
    // LIGHT
    // **************************************************
    let a_front_ao = Array::new(&front.ao, dim3);
    let a_rear_ao = Array::new(&rear.ao, dim3);
    let a_upper_ao = Array::new(&upper.ao, dim3);
    
    let a_front_diffuse = Array::new(&front.diffuse, dim3);
    let a_rear_diffuse = Array::new(&rear.diffuse, dim3);
    let a_upper_diffuse = Array::new(&upper.diffuse, dim3);
    
    let a_front_glossy = Array::new(&front.glossy, dim3);
    let a_rear_glossy = Array::new(&rear.glossy, dim3);
    let a_upper_glossy = Array::new(&upper.glossy, dim3);
    
    let mut a_ao = constant::<f32>(0_f32, dim3);
    let mut a_diffuse = constant::<f32>(0_f32, dim3);
    let mut a_glossy = constant::<f32>(0_f32, dim3);

    a_ao = select(&a_front_ao, &m_front, &a_ao);
    a_ao = select(&a_rear_ao, &m_rear, &a_ao);
    a_ao = select(&a_upper_ao, &m_upper, &a_ao);

    a_diffuse = select(&a_front_diffuse, &m_front, &a_diffuse);
    a_diffuse = select(&a_rear_diffuse, &m_rear, &a_diffuse);
    a_diffuse = select(&a_upper_diffuse, &m_upper, &a_diffuse);

    a_glossy = select(&a_front_glossy, &m_front, &a_glossy);
    a_glossy = select(&a_rear_glossy, &m_rear, &a_glossy);
    a_glossy = select(&a_upper_glossy, &m_upper, &a_glossy);
    
    let luma = Array::new(&[0.2126_f32, 0.7152_f32, 0.0722_f32], dim4!(1, 1, 3));

    a_ao = mul(&a_ao, &luma, true);
    a_diffuse = mul(&a_diffuse, &luma, true);
    a_glossy = mul(&a_glossy, &luma, true);

    a_ao = sum(&a_ao, 2);
    a_diffuse = sum(&a_diffuse, 2);
    a_glossy = sum(&a_glossy, 2);

    let mut a_light = join_many![2; &a_diffuse, &a_glossy, &a_ao];
    a_light = log2(&a_light);
    a_light = add(&a_light, &(12.473931188_f32), true);
    a_light = div(&a_light, &(25_f32), true);
    a_light = mul(&a_light, &(2.0_f32 * 255_f32), true);
    a_light = clamp(&a_light, &(0_f32), &(255_f32), true);
    a_light = reorder_v2(&a_light, 2, 0, Some(vec![1]));
    a_light.cast::<u8>().host::<u8>(&mut light);

    // **************************************************
    // MATTE
    // **************************************************
    let a_front_index = Array::new(&front.index, dim4);
    let a_rear_index = Array::new(&rear.index, dim4);
    let a_upper_index = Array::new(&upper.index, dim4);

    let a_front_matte = Array::new(&front.matte, dim4);
    let a_rear_matte = Array::new(&rear.matte, dim4);
    let a_upper_matte = Array::new(&upper.matte, dim4);

    // TODO: rank matte values
    // filter by id
    // sum_by_key
    // topk (k=4)

    // Map index values
    let mut a_index = constant(0, dim4);
    for (k, v) in map {
        let cond = eq(&a_index, k, true);
        replace(&mut a_index, &cond, &constant(*v, dim4));
    }

    // TODO: bit-pack index values
    let r_1 = view!(a_index[1:1:0, 1:1:0, 0:0:1]).cast::<u8>();
    let r_2 = view!(a_index[1:1:0, 1:1:0, 1:1:1]).cast::<u8>();
    let r_3 = view!(a_index[1:1:0, 1:1:0, 2:2:1]).cast::<u8>();
    let r_4 = view!(a_index[1:1:0, 1:1:0, 3:3:1]).cast::<u8>();

    let d = dim4!(size, size);
    let a_r = bitor(&r_1, &bitand(&r_2, &constant(0b11u8, d), false).shl(6_u8), false);
    let a_g = bitor(&bitand(&r_2, &constant(0b111100u8, d), false).shr(2_u8), &bitand(&r_3, &constant(0b1111u8, d), false).shl(4_u8), false);
    let a_b = bitor(&bitand(&r_3, &constant(0b110000u8, d), false).shr(4_u8), &r_4.shl(2_u8), false);

    let mut a_index = join_many![2; &a_r, &a_g, &a_b];
    a_index = reorder_v2(&a_index, 2, 0, Some(vec![1]));
    a_index.cast::<u8>().host::<u8>(&mut index);

    return (light, index, matte);
}


fn save_webp(path: PathBuf, size: u32, pixels: &Vec<u8>, compression: WebpCompressionType) {
    let img = match compression {
        WebpCompressionType::LOSSLESS => webp::Encoder::from_rgb(pixels, size, size).encode_lossless(),
        WebpCompressionType::LOSSY(quality) => webp::Encoder::from_rgb(pixels, size, size).encode(quality),
    };
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}


fn main() {
    let args = CliArgs::parse();
    
    let size = args.resolution;
    let front_dir = &args.front;
    let rear_dir = &args.rear;
    let upper_dir = &args.upper;
    let zmask_dir = &args.zmask;
    let light_dir = &args.light;
    let index_dir = &args.index;
    let matte_dir = &args.matte;

    let front_files = fs::read_dir(front_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let rear_files = fs::read_dir(rear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let upper_files = fs::read_dir(upper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zmask_files = fs::read_dir(zmask_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if front_files.len() != num_frames { panic!("Missing 'Front' files"); }
    if rear_files.len() != num_frames { panic!("Missing 'Rear' files"); }
    if upper_files.len() != num_frames { panic!("Missing 'Upper' files"); }
    if zmask_files.len() != num_frames { panic!("Missing 'ZMask' files"); }

    let map = get_asset_map();

    for frame in 0..num_frames {
        let f_front = &front_files[frame];
        let f_rear = &rear_files[frame];
        let f_upper = &upper_files[frame];
        let f_zmask = &zmask_files[frame];

        let front = read_foreground_exr(&f_front.path(), size);
        let rear = read_foreground_exr(&f_rear.path(), size);
        let upper = read_foreground_exr(&f_upper.path(), size);
        let zmask = image::open(f_zmask.path()).unwrap().to_rgb8().as_bytes().to_vec();

        let (light, index, matte) = composite(
            &map,
            front,
            rear,
            upper,
            &zmask,
            size as u64,
        );

        let path_out_light = light_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        // let path_out_index = index_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        // let path_out_matte = matte_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");

        save_webp(path_out_light, size, &light, WebpCompressionType::LOSSY(100.0));
        // save_webp(path_out_index, size, &index, WebpCompressionType::LOSSLESS);
        // save_webp(path_out_matte, size, &matte, WebpCompressionType::LOSSLESS);
    }
}
