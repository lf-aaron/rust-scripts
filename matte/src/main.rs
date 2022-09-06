use arrayfire::*;
use clap::Parser;
use std::io::{BufWriter, Write};
use std::fs;
use std::mem::{transmute};
use std::ops::{Not, Shl, Shr};
use std::path::{Path, PathBuf};
use exr::prelude::*;
use webp;


struct MatteStruct {
    resolution: usize,
    index: Vec<u32>,
    matte: Vec<f32>,
}

enum MattePass {
    INDEX,
    MATTE,
}

enum RGBAChannel {
    R,
    G,
    B,
    A,
}

enum WebpCompressionType {
    LOSSY(f32),
    LOSSLESS,
}


impl MatteStruct {
    fn new (resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            index: vec![0_u32; n * 4],
            matte: vec![0_f32; n * 4],
        }
    }
  
    fn set_channel(&mut self, channel_data: Vec<f32>, pass: MattePass, channel: RGBAChannel) {
      
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
            MattePass::INDEX => { self.index.splice(offset..offset+n, unsafe { channel_data.into_iter().map(|x| {transmute::<f32, u32>(x)}).collect::<Vec<u32>>()}); },
            // MattePass::INDEX => { self.index.splice(offset..offset+n, unsafe { transmute::<Vec<f32>, Vec<u32>>(channel_data) }); },
            MattePass::MATTE => { self.matte.splice(offset..offset+n, channel_data); },
        };
    }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    resolution: u32,

    #[clap(long, parse(from_os_str))]
    input: PathBuf,

    #[clap(long, parse(from_os_str))]
    index: PathBuf,

    #[clap(long, parse(from_os_str))]
    matte: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}


fn get_index_map() -> [(u32, f32); 32] {
    [
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
    ]

    // HashMap::from(array.map(|v | { ( unsafe { transmute::<f32, u32>(v.1) }, v.0) }))

}


fn read_matte_exr(path: &Path, resolution: u32) -> MatteStruct {

    // There are 12 channels in total.
    // Colors organized as (A, B, G, R)
    // 1) Combined
    // 2) Crypto00
    // 3) Crypto01
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

    let mut obj = MatteStruct::new(resolution as usize);

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };
    
    for (i, _) in channels.iter().enumerate() {
        match i {
            4 => obj.set_channel(f(&channels[i]), MattePass::MATTE, RGBAChannel::G),    // Crypto00.A
            5 => obj.set_channel(f(&channels[i]), MattePass::INDEX, RGBAChannel::G),    // Crypto00.B
            6 => obj.set_channel(f(&channels[i]), MattePass::MATTE, RGBAChannel::R),    // Crypto00.G
            7 => obj.set_channel(f(&channels[i]), MattePass::INDEX, RGBAChannel::R),    // Crypto00.R
            8 => obj.set_channel(f(&channels[i]), MattePass::MATTE, RGBAChannel::A),    // Crypto01.A
            9 => obj.set_channel(f(&channels[i]), MattePass::INDEX, RGBAChannel::A),    // Crypto01.B
            10 => obj.set_channel(f(&channels[i]), MattePass::MATTE, RGBAChannel::B),   // Crypto01.G
            11 => obj.set_channel(f(&channels[i]), MattePass::INDEX, RGBAChannel::B),   // Crypto01.R
            _ => {},
        };
    }

    return obj;
}


fn composite(
    arr: &[(u32, f32); 32],
    exr: MatteStruct,
    size: u64,
) -> (Vec<u8>, Vec<u8>) {
    
    let dim3 = dim4!(size, size, 3);
    let dim4 = dim4!(size, size, 4);

    let mut index = vec!(0; dim3.elements() as usize);
    let mut matte = vec!(0; dim3.elements() as usize);

    // Map index values
    let mut a_index = Array::new(&exr.index, dim4);
    for (k, v) in arr {
        let cond = eq(&a_index, &constant(unsafe { transmute::<f32, u32>(*v) }, dim4), false);
        replace(&mut a_index, &cond.not(), &constant(*k, dim4));
    }
    
    // Bit-pack
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


    // Matte
    let mut a_matte = Array::new(&exr.matte, dim4);
    let mut r_1 = view!(a_matte[1:1:0, 1:1:0, 1:1:1]);
    let mut r_2 = view!(a_matte[1:1:0, 1:1:0, 2:2:1]);
    let mut r_3 = view!(a_matte[1:1:0, 1:1:0, 3:3:1]);

    // TODO: Use 9 bits for rank 2?
    r_1 = clamp(&mul(&r_1, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);
    r_2 = clamp(&mul(&r_2, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);
    r_3 = clamp(&mul(&r_3, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);

    a_matte = join_many![2; &r_1, &r_2, &r_3];
    a_matte = reorder_v2(&a_matte, 2, 0, Some(vec![1]));
    a_matte.cast::<u8>().host::<u8>(&mut matte);

    return (index, matte);
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
    let in_dir = &args.input;
    let matte_dir = &args.matte;
    let index_dir = &args.index;
    let device = args.device;
    let overwrite = args.overwrite;

    set_backend(Backend::CUDA);
    set_device(device);

    let mut in_files = fs::read_dir(in_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if in_files.len() != num_frames { panic!("Missing 'Front' files"); }

    in_files.sort_by(|a, b| {a.file_name().cmp(&b.file_name())});

    let arr = get_index_map();

    for frame in 0..num_frames {
        let path_out_index = index_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        let path_out_matte = matte_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out_index.exists() && path_out_matte.exists() {
            continue;
        }
        
        let f_in = &in_files[frame];
        let exr = read_matte_exr(&f_in.path(), size);

        let (index, matte) = composite(&arr, exr, size as u64);

        save_webp(path_out_index, size, &index, WebpCompressionType::LOSSLESS);
        save_webp(path_out_matte, size, &matte, WebpCompressionType::LOSSLESS);
    }
}
