use arrayfire::*;
use clap::Parser;
use exr::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use util::{save_webp, WebpCompressionType};

fn preload(
    base_resolution: u32,
    level: u32,
    frame: u32,
    input_dir: &Path,
) -> (
    HashMap<&str, Vec<f32>>,
    HashMap<&str, Vec<f32>>,
    HashMap<&str, Vec<f32>>,
) {
    // let resolution = base_resolution * 2_u32.pow(level);

    let mut front_map: HashMap<&str, Vec<f32>> = HashMap::new();
    let mut rear_map: HashMap<&str, Vec<f32>> = HashMap::new();
    let mut upper_map: HashMap<&str, Vec<f32>> = HashMap::new();

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
        let path = input_dir
            .join(format!(
                "{}/{}/{}/{:0>4}",
                base_resolution,
                config,
                level,
                (121 + frame).to_string()
            ))
            .with_extension("exr");
        front_map.insert(config, read_depth_exr(&path));
        break;
    }

    for config in rear_configs {
        let path = input_dir
            .join(format!(
                "{}/{}/{}/{:0>4}",
                base_resolution,
                config,
                level,
                (121 + frame).to_string()
            ))
            .with_extension("exr");
        rear_map.insert(config, read_depth_exr(&path));
        break;
    }

    for config in upper_configs {
        let path = input_dir
            .join(format!(
                "{}/{}/{}/{:0>4}",
                base_resolution,
                config,
                level,
                (121 + frame).to_string()
            ))
            .with_extension("exr");
        upper_map.insert(config, read_depth_exr(&path));
        break;
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
                                    return;
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
    depth: PathBuf,

    #[clap(long, parse(from_os_str))]
    zrank: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
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

    let f = |ch: &AnyChannel<FlatSamples>| {
        match &ch.sample_data {
            exr::prelude::FlatSamples::F32(x) => x.to_owned(),
            _ => panic!("Unexpected channel type"),
        }
    };

    f(&channel.last().unwrap())
}


fn compute_z_rank(
    size: u64,
    z_front: &Vec<f32>,
    z_rear: &Vec<f32>,
    z_upper: &Vec<f32>,
) -> Vec<u8> {
    let dims = dim4!(size, size);

    let mask = constant::<f32>(1_f32, dim4!(3 ,3));
    let mut cond: Array<bool>;
    let mut z_rank = vec!(0; (3 * size * size) as usize);

    let mut depth_front = Array::new(z_front, dims);
    let mut depth_rear = Array::new(z_rear, dims);
    let mut depth_upper = Array::new(z_upper, dims);

    // Expand depth pass ~1px to account for pixel blur in matte pass
    let erode_front = erode(&depth_front, &mask);
    let erode_rear = erode(&depth_rear, &mask);
    let erode_upper = erode(&depth_upper, &mask);

    // Filter out pixels with no depth information
    cond = lt(&depth_front, &1.0, true);
    replace(&mut depth_front, &cond, &constant(0_f32, dims));

    cond = lt(&depth_rear, &1.0, true);
    replace(&mut depth_rear, &cond, &constant(0_f32, dims));
    
    cond = lt(&depth_upper, &1.0, true);
    replace(&mut depth_upper, &cond, &constant(0_f32, dims));

    // Choose max depth for each pixel
    cond = gt(&depth_front, &erode_front, false);
    replace(&mut depth_front, &cond, &erode_front);

    cond = gt(&depth_rear, &erode_rear, false);
    replace(&mut depth_rear, &cond, &erode_rear);
    
    cond = gt(&depth_upper, &erode_upper, false);
    replace(&mut depth_upper, &cond, &erode_upper);

    // Sort by depth
    let depth = join_many![2; &depth_front, &depth_rear, &depth_upper];
    let (_, mut rank) = sort_index(&depth, 2, false);

    rank = reorder_v2(&rank, 2, 0, Some(vec![1]));
    rank.cast::<u8>().host::<u8>(&mut z_rank);
    return z_rank;
}


fn main() {
    let args = CliArgs::parse();
    
    let frame = args.frame;
    let level = args.level;
    let base_resolution = args.base_resolution;
    let depth_dir = args.depth;
    let zrank_dir = args.zrank;
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
    let (front_map, rear_map, upper_map) = preload(base_resolution, level, frame, &depth_dir);

    for (config, front, rear, upper) in configs {
        let path_out = zrank_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out.exists() {
            continue;
        }

        let folder = path_out.parent().unwrap();
        if !folder.exists() { let _ = fs::create_dir_all(folder); }

        let z_front = front_map.get(front.as_str()).unwrap();
        let z_rear = rear_map.get(rear.as_str()).unwrap();
        let z_upper = upper_map.get(upper.as_str()).unwrap();

        let zrank = compute_z_rank(
            resolution as u64,
            z_front,
            z_rear,
            z_upper,
        );

        save_webp(path_out, resolution, &zrank, WebpCompressionType::LOSSLESS);
    }
}