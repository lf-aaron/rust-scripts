use arrayfire::*;
use clap::Parser;
use image::EncodableLayout;
use std::collections::HashMap;
use std::fs;
use std::mem::{transmute};
use std::ops::{Not, Shl, Shr};
use std::path::{Path, PathBuf};
use exr::prelude::*;
use util::{RGBAChannel, WebpCompressionType, save_webp};


struct MatteStruct {
    resolution: usize,
    index: Vec<u32>,
    matte: Vec<f32>,
}

enum MattePass {
    INDEX,
    MATTE,
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
            MattePass::MATTE => { self.matte.splice(offset..offset+n, channel_data); },
        };
    }
}


fn preload(base_resolution: u32, level: u32, frame: u32, input_dir: &Path) -> (HashMap<&str, MatteStruct>, HashMap<&str, MatteStruct>, HashMap<&str, MatteStruct>) {
    let resolution = base_resolution * 2_u32.pow(level);
    
    let mut front_map: HashMap<&str, MatteStruct> = HashMap::new();
    let mut rear_map: HashMap<&str, MatteStruct> = HashMap::new();
    let mut upper_map: HashMap<&str, MatteStruct> = HashMap::new();

    let front_configs = vec![
        "Front Std Com",
        "Front Std Com Ext",
        "Front Std Gov",
        "Front Std Gov Ext",
        "Front Std Sqr Com",
        "Front Std Sqr Com Ext",
        "Front Std Sqr Gov",
        "Front Std Sqr Gov Ext",
        "Front Tac Com",
        "Front Tac Com Ext",
        "Front Tac Gov",
        "Front Tac Gov Ext",
        "Front Tac Sqr Com",
        "Front Tac Sqr Com Ext",
        "Front Tac Sqr Gov",
        "Front Tac Sqr Gov Ext",
    ];

    let rear_configs = vec![
        "Rear 9mm",
        "Rear 9mm Bob",
        "Rear 9mm Bob FChk",
        "Rear 9mm Bob RChk",
        "Rear 9mm Bob RChk FChk",
        "Rear 9mm FChk",
        "Rear 9mm RChk",
        "Rear 9mm RChk FChk",
    ];

    let upper_configs = vec![
        "Upper .45 Com Novak",
        "Upper .45 Com Ext Novak",
        "Upper .45 Gov Novak",
        "Upper .45 Gov Ext Novak",
        "Upper 9mm Com Novak",
        "Upper 9mm Com Ext Novak",
        "Upper 9mm Gov Novak",
        "Upper 9mm Gov Ext Novak",
        "Upper 10mm Com Novak",
        "Upper 10mm Com Ext Novak",
        "Upper 10mm Gov Novak",
        "Upper 10mm Gov Ext Novak",
        "Upper .38 Com Novak",
        "Upper .38 Com Ext Novak",
        "Upper .38 Gov Novak",
        "Upper .38 Gov Ext Novak",
        "Upper .40 Com Novak",
        "Upper .40 Com Ext Novak",
        "Upper .40 Gov Novak",
        "Upper .40 Gov Ext Novak",
    ];

    for config in front_configs {
        let path = input_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        front_map.insert(config, read_matte_exr(&path, resolution));
    }

    for config in rear_configs {
        let path = input_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        rear_map.insert(config, read_matte_exr(&path, resolution));
    }

    for config in upper_configs {
        let path = input_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        upper_map.insert(config, read_matte_exr(&path, resolution));
    }

   (front_map, rear_map, upper_map)
}

struct ConfigOptions<'a> {
    style: Vec<&'a str>,
    guard: Vec<&'a str>,
    caliber: Vec<&'a str>,
    size: Vec<&'a str>,
    ext: Vec<&'a str>,
    rear: Vec<&'a str>,
    rchk: Vec<&'a str>,
    fchk: Vec<&'a str>,
}


fn get_name<'a>(v: Vec<&&'a str>) -> String {
    v.into_iter().map(|x| {x.to_owned()}).filter(|x| {!x.is_empty()}).collect::<Vec<&str>>().join(" ").trim_end().to_string()
}


fn get_configurations(configs: &mut Vec<(String, String, String, String)>, options: ConfigOptions) {
    let trig = "";
    let sight = "Novak";
    for style in &options.style {
        for guard in &options.guard {
            for caliber in &options.caliber {
                for size in &options.size {
                    for ext in &options.ext {
                        for rear in &options.rear {
                            for rchk in &options.rchk {
                                for fchk in &options.fchk {
                                    configs.push((
                                        get_name(vec![style, guard, caliber, size, ext, rear, rchk, fchk]),
                                        get_name(vec![&"Front", style, guard, size, ext]),
                                        get_name(vec![&"Rear", &"9mm", rear, rchk, fchk, &trig]),
                                        get_name(vec![&"Upper", caliber, size, ext, &sight]),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    level: u32,

    #[clap(long)]
    base_resolution: u32,

    #[clap(long)]
    
    frame: u32,

    #[clap(long, parse(from_os_str))]
    input: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

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
        (0, 46.93645477294922), // VOID
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
        (31, 0.0), // NONE
    ]
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
            // 0                                                                        // Combined.A
            // 1                                                                        // Combined.B
            // 2                                                                        // Combined.G
            // 3                                                                        // Combined.R
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
    front: &MatteStruct,
    rear: &MatteStruct,
    upper: &MatteStruct,
    zmask: &Vec<u8>,
    size: u64,
) -> (Vec<u8>, Vec<u8>) {
    
    let dim3 = dim4!(size, size, 3);
    let dim4 = dim4!(size, size, 4);

    let mut index = vec!(0; dim3.elements() as usize);
    let mut matte = vec!(0; dim3.elements() as usize);

    // Combine sections using zmask
    let mut a_zmask = Array::new(zmask, dim4!(3, size, size)).cast::<bool>();
    a_zmask = reorder_v2(&a_zmask, 1, 2, Some(vec![0]));

    // NOTE: `1:1:0` means all elements along axis
    let m_front = view!(a_zmask[1:1:0, 1:1:0, 0:0:1]);
    let m_rear = view!(a_zmask[1:1:0, 1:1:0, 1:1:1]);
    let m_upper = view!(a_zmask[1:1:0, 1:1:0, 2:2:1]);

    let a_front_index = Array::new(&front.index, dim4);
    let a_rear_index = Array::new(&rear.index, dim4);
    let a_upper_index = Array::new(&upper.index, dim4);
    
    let a_front_matte = Array::new(&front.matte, dim4);
    let a_rear_matte = Array::new(&rear.matte, dim4);
    let a_upper_matte = Array::new(&upper.matte, dim4);

    let mut a_index = constant::<u32>(0_u32, dim4);
    let mut a_matte = constant::<f32>(0_f32, dim4);

    a_index = select(&a_front_index, &m_front, &a_index);
    a_index = select(&a_rear_index, &m_rear, &a_index);
    a_index = select(&a_upper_index, &m_upper, &a_index);

    a_matte = select(&a_front_matte, &m_front, &a_matte);
    a_matte = select(&a_rear_matte, &m_rear, &a_matte);
    a_matte = select(&a_upper_matte, &m_upper, &a_matte);

    // Map index values
    let a_index_copy = a_index.copy();
    for (k, v) in arr {
        let cond = eq(&a_index_copy, &constant(unsafe { transmute::<f32, u32>(*v) }, dim4), false);
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
    let mut r_1 = view!(a_matte[1:1:0, 1:1:0, 1:1:1]);
    let mut r_2 = view!(a_matte[1:1:0, 1:1:0, 2:2:1]);
    let mut r_3 = view!(a_matte[1:1:0, 1:1:0, 3:3:1]);

    r_1 = clamp(&mul(&r_1, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);
    r_2 = clamp(&mul(&r_2, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);
    r_3 = clamp(&mul(&r_3, &(2.0_f32 * 255_f32), true), &(0_f32), &(255_f32), true);

    a_matte = join_many![2; &r_1, &r_2, &r_3];
    a_matte = reorder_v2(&a_matte, 2, 0, Some(vec![1]));
    a_matte.cast::<u8>().host::<u8>(&mut matte);

    return (index, matte);
}

fn main() {
    let args = CliArgs::parse();
    
    let frame = args.frame;
    let level = args.level;
    let base_resolution = args.base_resolution;
    let input_dir = args.input;
    let zmask_dir = args.zmask;
    let index_dir = &args.index;
    let matte_dir = &args.matte;
    let device = args.device;
    let overwrite = args.overwrite;

    set_backend(Backend::CUDA);
    set_device(device);

    let resolution = base_resolution * 2_u32.pow(level);

    let mut configs: Vec<(String, String, String, String)> = Vec::new();

    let options = ConfigOptions  {
        style: vec!["Std", "Tac"],
        guard: vec!["", "Sqr"],
        caliber: vec![".45", "9mm", "10mm", ".38", ".40"],
        size: vec!["Com", "Gov"],
        ext: vec!["", "Ext"],
        rear: vec!["", "Bob"],
        rchk: vec!["", "RChk"],
        fchk: vec!["", "FChk"],
    };

    get_configurations(&mut configs, options);
    let (front_map, rear_map, upper_map) = preload(base_resolution, level, frame, &input_dir);

    let arr = get_index_map();

    for (config, front, rear, upper) in configs {
        let path_out_index = index_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        let path_out_matte = matte_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out_index.exists() && path_out_matte.exists() {
            continue;
        }

        let index_folder = path_out_index.parent().unwrap();
        let matte_folder = path_out_matte.parent().unwrap();
        if !index_folder.exists() { let _ = fs::create_dir_all(index_folder); }
        if !matte_folder.exists() { let _ = fs::create_dir_all(matte_folder); }

        let zmask_path = zmask_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");

        let front_exr = front_map.get(front.as_str()).unwrap();
        let rear_exr = rear_map.get(rear.as_str()).unwrap();
        let upper_exr = upper_map.get(upper.as_str()).unwrap();

        let zmask = image::open(zmask_path).unwrap().to_rgb8().as_bytes().to_vec();

        let (index, matte) = composite(
            &arr,
            front_exr,
            rear_exr,
            upper_exr,
            &zmask,
            resolution as u64,
        );

        save_webp(path_out_index, resolution, &index, WebpCompressionType::LOSSLESS);
        save_webp(path_out_matte, resolution, &matte, WebpCompressionType::LOSSLESS);
    }
}
