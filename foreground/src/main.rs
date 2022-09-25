use arrayfire::*;
use clap::Parser;
use exr::prelude::*;
use image::{EncodableLayout};
use std::collections::HashMap;
use std::fs;
// use std::fs::{DirEntry, read_dir};
use std::path::{Path, PathBuf};
use util::{RGBAChannel, WebpCompressionType, save_webp};


struct ForegroundStruct {
    resolution: usize,
    ao: Vec<f32>,
    diffuse: Vec<f32>,
    glossy: Vec<f32>,
}

#[derive(Debug)]
enum ForegroundPass {
    AO,
    DIFFUSE,
    GLOSSY,
}

fn num_channels(pass: ForegroundPass) -> usize {
    match pass {
        ForegroundPass::AO => 3,
        ForegroundPass::DIFFUSE => 3,
        ForegroundPass::GLOSSY => 3,
    }
}


fn preload(base_resolution: u32, level: u32, frame: u32, foreground_dir: &Path) -> (HashMap<&str, ForegroundStruct>, HashMap<&str, ForegroundStruct>, HashMap<&str, ForegroundStruct>) {
    let resolution = base_resolution * 2_u32.pow(level);
    
    let mut front_map: HashMap<&str, ForegroundStruct> = HashMap::new();
    let mut rear_map: HashMap<&str, ForegroundStruct> = HashMap::new();
    let mut upper_map: HashMap<&str, ForegroundStruct> = HashMap::new();

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
        let path = foreground_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        front_map.insert(config, read_foreground_exr(&path, resolution));
        // front_map.insert(config, ForegroundStruct::new(0));
    }

    for config in rear_configs {
        let path = foreground_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        rear_map.insert(config, read_foreground_exr(&path, resolution));
        // rear_map.insert(config, ForegroundStruct::new(0));
    }

    for config in upper_configs {
        let path = foreground_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("exr");
        // println!("{:?} -> {:?}", path, path.exists());
        upper_map.insert(config, read_foreground_exr(&path, resolution));
        // upper_map.insert(config, ForegroundStruct::new(0));
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


impl ForegroundStruct {
    fn new (resolution: usize) -> Self {
        let n = resolution * resolution;
        Self {
            resolution,
            ao: vec![0_f32; n * num_channels(ForegroundPass::AO)],
            diffuse: vec![0_f32; n * num_channels(ForegroundPass::DIFFUSE)],
            glossy: vec![0_f32; n * num_channels(ForegroundPass::GLOSSY)],
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
        };
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
    foreground: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

    #[clap(long, parse(from_os_str))]
    light: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}


fn read_foreground_exr(path: &Path, resolution: u32) -> ForegroundStruct {

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

    let mut obj = ForegroundStruct::new(resolution as usize);

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };
    
    for (i, _) in channels.iter().enumerate() {
        match i {
            0 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::B),        // AO.B
            1 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::G),        // AO.G
            2 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::R),        // AO.R
            // 3                                                                              // Combined.A
            // 4                                                                              // Combined.B
            // 5                                                                              // Combined.G
            // 6                                                                              // Combined.R
            7 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::B),  // Diffuse.B
            8 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::G),  // Diffuse.G
            9 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::R),  // Diffuse.R
            10 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::B),   // Glossy.B
            11 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::G),   // Glossy.G
            12 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::R),   // Glossy.R
            _ => {},
        };
    }

    return obj;
}


fn composite(
    front: &ForegroundStruct,
    rear: &ForegroundStruct,
    upper: &ForegroundStruct,
    zmask: &Vec<u8>,
    size: u64,
) -> Vec<u8> {
    
    let dims = dim4!(size, size, 3);

    let mut light = vec!(0; dims.elements() as usize);

    // Combine sections using zmask
    let mut a_zmask = Array::new(zmask, dim4!(3, size, size)).cast::<bool>();
    a_zmask = reorder_v2(&a_zmask, 1, 2, Some(vec![0]));

    // NOTE: `1:1:0` means all elements along axis
    let m_front = view!(a_zmask[1:1:0, 1:1:0, 0:0:1]);
    let m_rear = view!(a_zmask[1:1:0, 1:1:0, 1:1:1]);
    let m_upper = view!(a_zmask[1:1:0, 1:1:0, 2:2:1]);

    let a_front_ao = Array::new(&front.ao, dims);
    let a_rear_ao = Array::new(&rear.ao, dims);
    let a_upper_ao = Array::new(&upper.ao, dims);
    
    let a_front_diffuse = Array::new(&front.diffuse, dims);
    let a_rear_diffuse = Array::new(&rear.diffuse, dims);
    let a_upper_diffuse = Array::new(&upper.diffuse, dims);
    
    let a_front_glossy = Array::new(&front.glossy, dims);
    let a_rear_glossy = Array::new(&rear.glossy, dims);
    let a_upper_glossy = Array::new(&upper.glossy, dims);
    
    let mut a_ao = constant::<f32>(0_f32, dims);
    let mut a_diffuse = constant::<f32>(0_f32, dims);
    let mut a_glossy = constant::<f32>(0_f32, dims);

    a_ao = select(&a_front_ao, &m_front, &a_ao);
    a_ao = select(&a_rear_ao, &m_rear, &a_ao);
    a_ao = select(&a_upper_ao, &m_upper, &a_ao);

    a_diffuse = select(&a_front_diffuse, &m_front, &a_diffuse);
    a_diffuse = select(&a_rear_diffuse, &m_rear, &a_diffuse);
    a_diffuse = select(&a_upper_diffuse, &m_upper, &a_diffuse);

    a_glossy = select(&a_front_glossy, &m_front, &a_glossy);
    a_glossy = select(&a_rear_glossy, &m_rear, &a_glossy);
    a_glossy = select(&a_upper_glossy, &m_upper, &a_glossy);
    
    // Convert to BW and log color space
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
    a_light = mul(&a_light, &(0.04_f32 * 2_f32 * 255_f32), true);
    a_light = clamp(&a_light, &(0_f32), &(255_f32), true);
    a_light = reorder_v2(&a_light, 2, 0, Some(vec![1]));
    a_light.cast::<u8>().host::<u8>(&mut light);

    return light;
}

fn main() {
    
    let args = CliArgs::parse();

    let frame = args.frame;
    let level = args.level;
    let base_resolution = args.base_resolution;
    let foreground_dir = args.foreground;
    let zmask_dir = args.zmask;
    let light_dir = &args.light;
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
    let (front_map, rear_map, upper_map) = preload(base_resolution, level, frame, &foreground_dir);

    for (config, front, rear, upper) in configs {
        let path_out = light_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out.exists() {
            continue;
        }

        let folder = path_out.parent().unwrap();
        if !folder.exists() {
            let _ = fs::create_dir_all(folder);
        }

        let zmask_path = zmask_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");

        let front_exr = front_map.get(front.as_str()).unwrap();
        let rear_exr = rear_map.get(rear.as_str()).unwrap();
        let upper_exr = upper_map.get(upper.as_str()).unwrap();

        let zmask = image::open(zmask_path).unwrap().to_rgb8().as_bytes().to_vec();

        let light = composite(
            front_exr,
            rear_exr,
            upper_exr,
            &zmask,
            resolution as u64,
        );

        save_webp(path_out, resolution, &light, WebpCompressionType::LOSSLESS);
    }
}
