mod config;
mod light;
mod mask;

use arrayfire::*;
use clap::Parser;
use config::SectionType;

use crate::config::Section;
// use image::EncodableLayout;
use std::collections::HashMap;
use std::fs;

// use std::fs::{DirEntry, read_dir};
use std::path::{Path, PathBuf};
use util::{save_webp, WebpCompressionType};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(long)]
    frame: u32,

    #[clap(long)]
    level: u32,

    #[clap(long)]
    resolution: u32,

    #[clap(long, parse(from_os_str))]
    input_dir: PathBuf,

    #[clap(long, parse(from_os_str))]
    output_dir: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}

fn preload(resolution: u32, level: u32, frame: u32, base_dir: &Path, section_type: &SectionType) -> HashMap<String, (light::EXRData, mask::EXRData)> {
    let mut map = HashMap::<String, (light::EXRData, mask::EXRData)>::new();

    for section in Section::iterator().filter(|&x| x.section_type() == *section_type) {
        for config in section.configurations() {
            let light_path = base_dir.join(format!("Diffuse/{}/{}/{}/{:0>4}", config, resolution, level, (121 + frame).to_string())).with_extension("exr");
            let mask_path = base_dir.join(format!("Mask/{}/{}/{}/{:0>4}", config, resolution, level, (121 + frame).to_string())).with_extension("exr");
            let light_exr = light::read_exr(&light_path, resolution);
            let mask_exr = mask::read_exr(&mask_path, resolution);
            map.insert(config, (light_exr, mask_exr));
        }
    }

    return map;
}

// fn main() {
//     let configs = Section::Barrel.configurations();
//     for config in &configs {
//         println!("{:?}", config);
//     }
// }

// fn composite(
//     front: &LightStruct,
//     rear: &LightStruct,
//     upper: &LightStruct,
//     zmask: &Vec<u8>,
//     size: u64,
// ) -> Vec<u8> {
//     let dims = dim4!(size, size, 3);

//     let mut light = vec![0; dims.elements() as usize];

//     // Combine sections using zmask
//     let mut a_zmask = Array::new(zmask, dim4!(3, size, size)).cast::<bool>();
//     a_zmask = reorder_v2(&a_zmask, 1, 2, Some(vec![0]));

//     // NOTE: `1:1:0` means all elements along axis
//     let m_front = view!(a_zmask[1:1:0, 1:1:0, 0:0:1]);
//     let m_rear = view!(a_zmask[1:1:0, 1:1:0, 1:1:1]);
//     let m_upper = view!(a_zmask[1:1:0, 1:1:0, 2:2:1]);

//     let a_front_ao = Array::new(&front.ao, dims);
//     let a_rear_ao = Array::new(&rear.ao, dims);
//     let a_upper_ao = Array::new(&upper.ao, dims);

//     let a_front_diffuse = Array::new(&front.diffuse, dims);
//     let a_rear_diffuse = Array::new(&rear.diffuse, dims);
//     let a_upper_diffuse = Array::new(&upper.diffuse, dims);

//     let a_front_glossy = Array::new(&front.glossy, dims);
//     let a_rear_glossy = Array::new(&rear.glossy, dims);
//     let a_upper_glossy = Array::new(&upper.glossy, dims);

//     let mut a_ao = constant::<f32>(0_f32, dims);
//     let mut a_diffuse = constant::<f32>(0_f32, dims);
//     let mut a_glossy = constant::<f32>(0_f32, dims);

//     a_ao = select(&a_front_ao, &m_front, &a_ao);
//     a_ao = select(&a_rear_ao, &m_rear, &a_ao);
//     a_ao = select(&a_upper_ao, &m_upper, &a_ao);

//     a_diffuse = select(&a_front_diffuse, &m_front, &a_diffuse);
//     a_diffuse = select(&a_rear_diffuse, &m_rear, &a_diffuse);
//     a_diffuse = select(&a_upper_diffuse, &m_upper, &a_diffuse);

//     a_glossy = select(&a_front_glossy, &m_front, &a_glossy);
//     a_glossy = select(&a_rear_glossy, &m_rear, &a_glossy);
//     a_glossy = select(&a_upper_glossy, &m_upper, &a_glossy);

//     // Convert to BW and log color space
//     let luma = Array::new(&[0.2126_f32, 0.7152_f32, 0.0722_f32], dim4!(1, 1, 3));

//     a_ao = mul(&a_ao, &luma, true);
//     a_diffuse = mul(&a_diffuse, &luma, true);
//     a_glossy = mul(&a_glossy, &luma, true);

//     a_ao = sum(&a_ao, 2);
//     a_diffuse = sum(&a_diffuse, 2);
//     a_glossy = sum(&a_glossy, 2);

//     let mut a_light = join_many![2; &a_diffuse, &a_glossy, &a_ao];
//     a_light = log2(&a_light);
//     a_light = add(&a_light, &(12.473931188_f32), true);
//     a_light = mul(&a_light, &(0.04_f32 * 2_f32 * 255_f32), true);
//     a_light = clamp(&a_light, &(0_f32), &(255_f32), true);
//     a_light = reorder_v2(&a_light, 2, 0, Some(vec![1]));
//     a_light.cast::<u8>().host::<u8>(&mut light);

//     return light;
// }

//(Vec<u8>, Vec<u8>, Vec<u8>)
fn composite(
    size: u64,
    exr_data: &HashMap::<String, (light::EXRData, mask::EXRData)>,
    config: &Vec<(config::Modifier, Option<&str>)>,
    section_type: SectionType,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {

    let all = seq!();
    let dim3 = dim4!(size, size, 3);
    let dim4 = dim4!(size, size, 4);

    let mut light = vec!(0; dim3.elements() as usize);
    let mut index = vec!(0; dim3.elements() as usize);
    let mut matte = vec!(0; dim3.elements() as usize);

    let sections = Section::iterator().filter(|x| x.section_type() == section_type).map(|x| x.matching_config(config)).collect::<Vec<String>>();

    // Initialize data arrays
    let n = (size * size) as usize;
    let k = sections.len();
    let mut v_depth = vec![0_f32; 1 * k * n];
    let mut v_index = vec![0_u32; 4 * k * n];
    let mut v_matte = vec![0_f32; 4 * k * n];
    let mut v_light = vec![0_f32; 3 * k * n];
    
    for (i, section) in sections.iter().enumerate() {
        let (light_exr, mask_exr) = exr_data.get(section).unwrap();
        v_depth.splice(i*n..(i+1)*n, mask_exr.depth.clone());
        v_index.splice(i*n*4..(i+1)*n*4, mask_exr.index.clone());
        v_matte.splice(i*n*4..(i+1)*n*4, mask_exr.matte.clone());

        v_light.splice((i+0)*n..(i+1)*n, light_exr.diffuse.clone());
        v_light.splice((i+1)*n..(i+2)*n, light_exr.glossy.clone());
        v_light.splice((i+2)*n..(i+3)*n, light_exr.ao.clone());
    }

    // let depth = sections.iter().map(|x| exr_data.get(x).unwrap().1.depth.clone()).collect::<Vec<Vec<f32>>>().concat();
    let a_depth = Array::new(&v_depth, dim4!(k as u64, 1, size, size));
    let a_index = Array::new(&v_index, dim4!(k as u64, 4, size, size));
    let a_matte = Array::new(&v_matte, dim4!(k as u64, 4, size, size));
    let a_light = Array::new(&v_light, dim4!(k as u64, 3, size, size));

    // Rank sections by z depth
    // TODO: ascending or descending?
    let (_, z_rank) = sort_index(&a_depth, 0, false);

    let mut c = constant::<f32>(0_f32, dim4!(mask::index_map.len() as u64, size, size));
    let mut l = constant::<f32>(0_f32, dim4!(3, size, size));

    for i in 0..k {
        let d = Seq::new(i as u32, i as u32, 1);
        let z_index = view!(z_rank[d, all, all]);

        let index_i = view!(a_index[z_index, all, all, all]);
        let matte_i = view!(a_matte[z_index, all, all, all]);

        // TODO: filter out 'NONE' and 'VOID'
        let void = or(&eq(&a_index, &mask::index_map.get(0).unwrap().1, true), &eq(&a_index, &mask::index_map.get(0).unwrap().1, true), false);
        let alpha = sum(&matte_i, 0);



        // let index_i = view!(a_index[z_rank, 1:1:0, 1:1:0, 1:1:0]);
        // let mut matte_i = a_matte[i];

        // matte_i = mul(&matte_i, &alpha, true);
        // let alpha_i = sum(&matte_i, 0);
        // alpha = sub(&alpha, )

    }

    // Map index values
    // let a_index_copy = a_index.copy();
    // for (k, v) in arr {
    //     let cond = eq(&a_index_copy, &constant(unsafe { transmute::<f32, u32>(*v) }, dim4), false);
    //     replace(&mut a_index, &cond.not(), &constant(*k, dim4));
    // }

    // let data = vec![];
    // get section configs from config
    // for section in Section::iterator().filter(|x| x.section_type() == section_type) {
    //     let section_config = section.matching_config(config);
    //     data.push(hashmap.get(&section_config).unwrap());
    // }

    return (light, index, matte);
}

fn main() {
    
    let args = CliArgs::parse();

    let frame = args.frame;
    let level = args.level;
    let base_resolution = args.resolution;
    let input_dir = args.input_dir;
    let output_dir = args.output_dir;
    let device = args.device;
    let overwrite = args.overwrite;

    set_backend(Backend::CUDA);
    set_device(device);

    let resolution = base_resolution * 2_u32.pow(level);

    for section_type in SectionType::iterator() {
        let hashmap = preload(base_resolution, level, frame, &input_dir, section_type);
        for config in section_type.configurations() {
            let config_name = config.iter().filter_map(|x| x.1).collect::<Vec<&str>>().join("-").as_str().to_ascii_lowercase();
            let base_out_path = output_dir.join(format!("{}/{}/{}/{:0>4}", config_name, base_resolution, level, (121 + frame).to_string()));

            let path_out_light = base_out_path.join("light").with_extension("webp");
            let path_out_index = base_out_path.join("index").with_extension("webp");
            let path_out_matte = base_out_path.join("matte").with_extension("webp");

            if !overwrite && path_out_light.exists() && path_out_index.exists() && path_out_matte.exists() { continue; }
            
            if !base_out_path.exists() { let _ = fs::create_dir_all(base_out_path); }

            let (light, matte, index) = composite(resolution as u64, &hashmap, &config, section_type.clone());

            save_webp(path_out_light, resolution, &light, WebpCompressionType::LOSSLESS);
            save_webp(path_out_index, resolution, &index, WebpCompressionType::LOSSLESS);
            save_webp(path_out_matte, resolution, &matte, WebpCompressionType::LOSSLESS);
            break // TESTING
        }

        break // TESTING
    }

    // let upper_hashmap = preload(base_resolution, level, frame, &input_dir, SectionType::Upper);
    for config in SectionType::Upper.configurations() {
        println!("{:?}", config::config_name(SectionType::Upper.name(), config.iter().map(|x| x.1).collect()));
        for section in Section::iterator().filter(|x| x.section_type() == SectionType::Upper) {
            let section_config = section.matching_config(&config);
            println!("  {:?}", section_config);
        }
        // composite(&upper_hashmap, &config, SectionType::Upper);
    }

    return ();

    // let lower_hashmap = preload(base_resolution, level, frame, &input_dir, SectionType::Lower);
    // for config in SectionType::Lower.configurations() {
        
    // }
}



//     let mut configs: Vec<(String, String, String, String)> = Vec::new();

//     let options = ConfigOptions  {
//         style: vec!["Std", "Tac"],
//         guard: vec!["", "Sqr"],
//         caliber: vec![".45", "9mm", "10mm", ".38", ".40"],
//         size: vec!["Com", "Gov"],
//         ext: vec!["", "Ext"],
//         rear: vec!["", "Bob"],
//         rchk: vec!["", "RChk"],
//         fchk: vec!["", "FChk"],
//     };

//     get_configurations(&mut configs, options);
//     let (front_map, rear_map, upper_map) = preload(base_resolution, level, frame, &foreground_dir);

//     for (config, front, rear, upper) in configs {
//         let path_out = light_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");
//         if !overwrite && path_out.exists() {
//             continue;
//         }

//         let folder = path_out.parent().unwrap();
//         if !folder.exists() {
//             let _ = fs::create_dir_all(folder);
//         }

//         let zmask_path = zmask_dir.join(format!("{}/{}/{}/{:0>4}", base_resolution, config, level, (121 + frame).to_string())).with_extension("webp");

//         let front_exr = front_map.get(front.as_str()).unwrap();
//         let rear_exr = rear_map.get(rear.as_str()).unwrap();
//         let upper_exr = upper_map.get(upper.as_str()).unwrap();

//         let zmask = image::open(zmask_path).unwrap().to_rgb8().as_bytes().to_vec();

//         let light = composite(
//             front_exr,
//             rear_exr,
//             upper_exr,
//             &zmask,
//             resolution as u64,
//         );

//         save_webp(path_out, resolution, &light, WebpCompressionType::LOSSLESS);
//     }
// }
