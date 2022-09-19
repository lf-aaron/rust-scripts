use arrayfire::*;
use clap::Parser;
use exr::prelude::*;
use image::{EncodableLayout};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use util::{RGBAChannel, WebpCompressionType, save_webp};


struct MetalStruct {
    resolution: usize,
    glossy: Vec<f32>,
}


fn preload<'a>(base_resolution: u32, level: u32, frame: u32, raw_dir: &'a Path, polish_dir: &'a Path) -> (HashMap<&'a str, MetalStruct>, HashMap<&'a str, MetalStruct>, HashMap<&'a str, MetalStruct>, HashMap<&'a str, MetalStruct>, HashMap<&'a str, MetalStruct>, HashMap<&'a str, MetalStruct>) {
    let resolution = base_resolution * 2_u32.pow(level);
    
    let mut front_map_raw: HashMap<&str, MetalStruct> = HashMap::new();
    let mut rear_map_raw: HashMap<&str, MetalStruct> = HashMap::new();
    let mut upper_map_raw: HashMap<&str, MetalStruct> = HashMap::new();

    let mut front_map_polish: HashMap<&str, MetalStruct> = HashMap::new();
    let mut rear_map_polish: HashMap<&str, MetalStruct> = HashMap::new();
    let mut upper_map_polish: HashMap<&str, MetalStruct> = HashMap::new();

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
        let path = format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string());
        let path_raw = raw_dir.join(&path).with_extension("exr");
        let path_polish = polish_dir.join(&path).with_extension("exr");
        println!("{:?} -> {:?}", path_raw, path_raw.exists());
        println!("{:?} -> {:?}", path_polish, path_polish.exists());
        front_map_raw.insert(config, read_metal_exr(&path_raw, resolution));
        front_map_polish.insert(config, read_metal_exr(&path_polish, resolution));
    }

    for config in rear_configs {
        let path = format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string());
        let path_raw = raw_dir.join(&path).with_extension("exr");
        let path_polish = polish_dir.join(&path).with_extension("exr");
        println!("{:?} -> {:?}", path_raw, path_raw.exists());
        println!("{:?} -> {:?}", path_polish, path_polish.exists());
        rear_map_raw.insert(config, read_metal_exr(&path_raw, resolution));
        rear_map_polish.insert(config, read_metal_exr(&path_polish, resolution));
    }

    for config in upper_configs {
        let path = format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string());
        let path_raw = raw_dir.join(&path).with_extension("exr");
        let path_polish = polish_dir.join(&path).with_extension("exr");
        println!("{:?} -> {:?}", path_raw, path_raw.exists());
        println!("{:?} -> {:?}", path_polish, path_polish.exists());
        upper_map_raw.insert(config, read_metal_exr(&path_raw, resolution));
        upper_map_polish.insert(config, read_metal_exr(&path_polish, resolution));
    }

   (front_map_raw, rear_map_raw, upper_map_raw, front_map_polish, rear_map_polish, upper_map_polish)
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


impl MetalStruct {
    fn new (resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            glossy: vec![0_f32; n * 3],
        }
    }

    
    fn set_channel(&mut self, channel_data: Vec<f32>, channel: RGBAChannel) {
        
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

        self.glossy.splice(offset..offset+n, channel_data);
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
    raw: PathBuf,

    #[clap(long, parse(from_os_str))]
    polish: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

    #[clap(long, parse(from_os_str))]
    metal: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}


fn read_metal_exr(path: &Path, resolution: u32) -> MetalStruct {

    // There are 7 channels in total.
    // Colors organized as (A, B, G, R), but some channels (AO, Diffuse, Glossy) do not contain alpha
    // 1) Combined
    // 2) Glossy
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

    let mut obj = MetalStruct::new(resolution as usize);

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };
    
    for (i, _) in channels.iter().enumerate() {
        match i {
            4 => obj.set_channel(f(&channels[i]), RGBAChannel::B),        // AO.B
            5 => obj.set_channel(f(&channels[i]), RGBAChannel::G),        // AO.G
            6 => obj.set_channel(f(&channels[i]), RGBAChannel::R),        // AO.R
            _ => {},
        };
    }

    return obj;
}


fn composite(
    front_raw: &MetalStruct,
    rear_raw: &MetalStruct,
    upper_raw: &MetalStruct,
    front_polish: &MetalStruct,
    rear_polish: &MetalStruct,
    upper_polish: &MetalStruct,
    zmask: &Vec<u8>,
    size: u64,
) -> Vec<u8> {
    
    let dims = dim4!(size, size, 3);

    let mut metal = vec!(0; dims.elements() as usize);

    let mut a_zmask = Array::new(zmask, dim4!(3, size, size)).cast::<bool>();
    a_zmask = reorder_v2(&a_zmask, 1, 2, Some(vec![0]));

    // NOTE: `1:1:0` means all elements along axis
    let m_front = view!(a_zmask[1:1:0, 1:1:0, 0:0:1]);
    let m_rear = view!(a_zmask[1:1:0, 1:1:0, 1:1:1]);
    let m_upper = view!(a_zmask[1:1:0, 1:1:0, 2:2:1]);

    let a_front_raw = Array::new(&front_raw.glossy, dims);
    let a_rear_raw = Array::new(&rear_raw.glossy, dims);
    let a_upper_raw = Array::new(&upper_raw.glossy, dims);

    let a_front_polish = Array::new(&front_polish.glossy, dims);
    let a_rear_polish = Array::new(&rear_polish.glossy, dims);
    let a_upper_polish = Array::new(&upper_polish.glossy, dims);
    
    let mut a_raw = constant::<f32>(0_f32, dims);
    let mut a_polish = constant::<f32>(0_f32, dims);

    a_raw = select(&a_front_raw, &m_front, &a_raw);
    a_raw = select(&a_rear_raw, &m_rear, &a_raw);
    a_raw = select(&a_upper_raw, &m_upper, &a_raw);

    a_polish = select(&a_front_polish, &m_front, &a_polish);
    a_polish = select(&a_rear_polish, &m_rear, &a_polish);
    a_polish = select(&a_upper_polish, &m_upper, &a_polish);
    
    let luma = Array::new(&[0.2126_f32, 0.7152_f32, 0.0722_f32], dim4!(1, 1, 3));

    a_raw = mul(&a_raw, &luma, true);
    a_polish = mul(&a_polish, &luma, true);

    a_raw = sum(&a_raw, 2);
    a_polish = sum(&a_polish, 2);

    let temp = constant::<f32>(0_f32, dim4!(size, size, 1));
    let mut a_metal = join_many![2; &a_raw, &a_polish, &temp];
    a_metal = log2(&a_metal);
    a_metal = add(&a_metal, &(12.473931188_f32), true);
    a_metal = mul(&a_metal, &(0.04_f32 * 2_f32 * 255_f32), true);
    a_metal = clamp(&a_metal, &(0_f32), &(255_f32), true);
    a_metal = reorder_v2(&a_metal, 2, 0, Some(vec![1]));
    a_metal.cast::<u8>().host::<u8>(&mut metal);

    return metal;
}

fn main() {
    
    let args = CliArgs::parse();

    let frame = args.frame;
    let level = args.level;
    let base_resolution = args.base_resolution;
    let raw_dir = args.raw;
    let polish_dir = args.polish;
    let zmask_dir = args.zmask;
    let metal_dir = &args.metal;
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
    let (front_map_raw, rear_map_raw, upper_map_raw, front_map_polish, rear_map_polish, upper_map_polish) = preload(base_resolution, level, frame, &raw_dir, &polish_dir);

    for (config, front, rear, upper) in configs {
        let path_out = metal_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out.exists() {
            continue;
        }

        let folder = path_out.parent().unwrap();
        if !folder.exists() {
            let _ = fs::create_dir_all(folder);
        }

        let zmask_path = zmask_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");

        let front_exr_raw = front_map_raw.get(front.as_str()).unwrap();
        let rear_exr_raw = rear_map_raw.get(rear.as_str()).unwrap();
        let upper_exr_raw = upper_map_raw.get(upper.as_str()).unwrap();
        let front_exr_polish = front_map_polish.get(front.as_str()).unwrap();
        let rear_exr_polish = rear_map_polish.get(rear.as_str()).unwrap();
        let upper_exr_polish = upper_map_polish.get(upper.as_str()).unwrap();

        let zmask = image::open(zmask_path).unwrap().to_rgb8().as_bytes().to_vec();

        let metal = composite(
            front_exr_raw,
            rear_exr_raw,
            upper_exr_raw,
            front_exr_polish,
            rear_exr_polish,
            upper_exr_polish,
            &zmask,
            resolution as u64,
        );

        save_webp(path_out, resolution, &metal, WebpCompressionType::LOSSLESS);
    }
}
