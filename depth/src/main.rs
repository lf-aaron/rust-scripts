use arrayfire::*;
use clap::Parser;
use exr::prelude::*;
use std::io::{BufWriter, Write};
use std::fs;
use std::path::{Path, PathBuf};
use std::ops::Neg;
use image::codecs::png;
use image::codecs::png::{CompressionType, FilterType};
use image::ColorType::Rgba8;
use image::ImageEncoder;
use webp;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    resolution: u32,

    #[clap(long)]
    config: String,

    #[clap(long, parse(from_os_str))]
    zfront: PathBuf,

    #[clap(long, parse(from_os_str))]
    zrear: PathBuf,

    #[clap(long, parse(from_os_str))]
    zupper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zplane: PathBuf,

    #[clap(long, parse(from_os_str))]
    zrank: PathBuf,
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
    
    match &channel.last().unwrap().sample_data {
        exr::prelude::FlatSamples::F32(x) => x.to_owned(),
        _ => panic!("Unexpected channel type"),
    }
}


fn rank_depth(a: &Array<f32>) -> Vec<u8> {
    let mask = arrayfire::constant::<f32>(1.0, arrayfire::Dim4::new(&[3, 3, 1, 1]));
    let mut x = a.copy().neg();
    x = arrayfire::dilate(&x, &mask);
    let (_, i) = arrayfire::sort_index(&x, 2, false);
    let mut buffer = vec!(0; a.dims().elements() as usize);
    let casted_array = i.cast::<u8>();
    let ordered_array = reorder_v2(&casted_array, 2, 0, Some(vec![1]));
    ordered_array.host::<u8>(&mut buffer);
    return buffer;
}


fn save_webp(path: PathBuf, size: u32, pixels: &Vec<u8>) {
    let img = webp::Encoder::from_rgba(pixels, size, size).encode_lossless();
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}

fn save_png(path: PathBuf, size: u32, pixels: &Vec<u8>) {
    let buffered_file_write = &mut BufWriter::new(fs::File::create(path).unwrap());
    png::PngEncoder::new_with_quality(
        buffered_file_write,
        CompressionType::Best,
        FilterType::NoFilter
    )
    .write_image(
        pixels,
        size as u32,
        size as u32,
        Rgba8
    )
    .unwrap();
}

fn main() {
    let args: CliArgs = CliArgs::parse();
    
    let size = args.resolution;
    let config = &args.config;
    let zfront_dir = &args.zfront;
    let zrear_dir = &args.zrear;
    let zupper_dir = &args.zupper;
    let zplane_path = &args.zplane;
    let zrank_dir = &args.zrank;

    let zplane_files = fs::read_dir(zplane_path).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zfront_files = fs::read_dir(zfront_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zrear_files = fs::read_dir(zrear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zupper_files = fs::read_dir(zupper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if zfront_files.len() != num_frames { panic!("Missing 'Z Front' files"); }
    if zrear_files.len() != num_frames { panic!("Missing 'Z Rear' files"); }
    if zupper_files.len() != num_frames { panic!("Missing 'Z Upper' files"); }

    let dims = Dim4::new(&[size as u64, size as u64, 4, 1]);
    let mut array = vec![0_f32; size as usize * size as usize * 4];
    // let zplane = read_depth_exr(&zplane_path);

    for i in 0..num_frames {
        let f_zplane = &zplane_files[i];
        let f_zfront = &zfront_files[i];
        let f_zrear = &zrear_files[i];
        let f_zupper = &zupper_files[i];

        // let zfront = read_depth_exr(&f_zfront.path());
        // let zrear = read_depth_exr(&f_zrear.path());
        // let zupper = read_depth_exr(&f_zupper.path());
        array.splice(3*size as usize..4*size as usize, read_depth_exr(&f_zplane.path()));
        array.splice(0*size as usize..1*size as usize, read_depth_exr(&f_zfront.path()));
        array.splice(1*size as usize..2*size as usize, read_depth_exr(&f_zrear.path()));
        array.splice(2*size as usize..3*size as usize, read_depth_exr(&f_zupper.path()));

        let zrank = rank_depth(&Array::new(&array , dims));

        save_png(zrank_dir.join(format!("{:0>4}", (121 + i).to_string())).with_extension("png"), size, &zrank);
    }

}