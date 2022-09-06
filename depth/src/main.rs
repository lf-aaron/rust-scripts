use arrayfire::*;
use clap::Parser;
use exr::prelude::*;
use std::io::{BufWriter, Write};
use std::fs;
use std::path::{Path, PathBuf};
use std::ops::{Not};
use webp;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long)]
    resolution: u32,

    #[clap(long, parse(from_os_str))]
    zfront: PathBuf,

    #[clap(long, parse(from_os_str))]
    zrear: PathBuf,

    #[clap(long, parse(from_os_str))]
    zupper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zplane: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}


fn read_depth_exr(path: &Path, v: &mut Vec<f32>) {
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
        // exr::prelude::FlatSamples::F32(x) => v.splice(.., x.to_owned()),
        exr::prelude::FlatSamples::F32(samples) => for (i, x) in samples.iter().enumerate() { v[i] = *x },
        _ => panic!("Unexpected channel type"),
    };
}

fn depth_mask(frame: usize, z_front: &Vec<f32>, z_rear: &Vec<f32>, z_upper: &Vec<f32>, z_plane: &Vec<f32>, size: u64) -> Vec<u8> {
    let dims = dim4!(size, size);
    let batch = false;
    let mask = constant::<bool>(true, dim4!(3, 3));
    
    let a_front = Array::new(z_front, dims);
    let a_rear = Array::new(z_rear, dims);
    let a_upper = Array::new(z_upper, dims);
    let a_plane = Array::new(z_plane, dims);

    let f_r = lt(&a_front, &a_rear, batch);
    let f_u = lt(&a_front, &a_upper, batch);
    let r_u = lt(&a_rear, &a_upper, batch);
    let r_f = lt(&a_rear, &a_front, batch);
    let u_f = lt(&a_upper, &a_front, batch);
    let u_r = lt(&a_upper, &a_rear, batch);

    let mut m_front = f_r & f_u;
    let mut m_rear = r_f & r_u;
    let m_upper = u_f & u_r;

    let r_p = match frame % 24 {
        // Right side view
        0 => {
            let n = dims[0] as i32;
            let p = gt(&range::<i32>(dim4!(n as u64), 0), &((n * 49/50)>>1), true);
            and(&m_rear, &p, true)
        },

        // Facing away
        1..=11 => {
            let p = ge(&a_rear, &a_plane, true);
            and(&m_rear, &p, batch)
        },

        // Left side view
        12 => {
            let n = dims[0] as i32;
            let p = le(&range::<i32>(dim4!(n as u64), 0), &((n * 51/50)>>1), true);
            and(&m_rear, &p, true)
        },

        // Facing toward
        13..=23 => {
            let p = lt(&a_rear, &a_plane, true);
            and(&m_rear, &p, batch)
        },
        _ => panic!("This should never happen")
    };

    m_front = or(&m_front, &r_p, batch);
    m_rear = and(&m_rear, &r_p.not(), batch);

    let d_front = and(&dilate(&m_front, &mask), &lt(&or(&m_rear, &m_upper, batch), &1, true), batch);
    let mut d_rear = and(&dilate(&m_rear, &mask), &lt(&or(&m_front, &m_upper, batch), &1, true), batch);
    let mut d_upper = and(&dilate(&m_upper, &mask), &lt(&or(&m_front, &m_rear, batch), &1, true), batch);

    d_upper = and(&d_upper, &d_rear.not(), batch);
    d_upper = and(&d_upper, &d_front.not(), batch);
    d_rear = and(&d_rear, &d_front.not(), batch);

    let mut buffer = vec!(0; 3 * dims.elements() as usize);
    let mut ar = join_many![2; &d_front, &d_rear, &d_upper];
    ar = reorder_v2(&ar, 2, 0, Some(vec![1]));
    ar.cast::<u8>().host::<u8>(&mut buffer);

    return buffer;
}


fn save_webp(path: PathBuf, size: u32, pixels: &Vec<u8>) {
    let img = webp::Encoder::from_rgb(pixels, size, size).encode_lossless();
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}

fn main() {
    let args: CliArgs = CliArgs::parse();
    
    let size = args.resolution;
    let zfront_dir = &args.zfront;
    let zrear_dir = &args.zrear;
    let zupper_dir = &args.zupper;
    let zplane_path = &args.zplane;
    let zmask_dir = &args.zmask;
    let device = args.device;
    let overwrite = args.overwrite;

    set_backend(Backend::CUDA);
    set_device(device);

    let mut zfront_files = fs::read_dir(zfront_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let mut zplane_files = fs::read_dir(zplane_path).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let mut zrear_files = fs::read_dir(zrear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let mut zupper_files = fs::read_dir(zupper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if zfront_files.len() != num_frames { panic!("Missing 'Z Front' files"); }
    if zplane_files.len() != num_frames { panic!("Missing 'Z Front' files"); }
    if zrear_files.len() != num_frames { panic!("Missing 'Z Rear' files"); }
    if zupper_files.len() != num_frames { panic!("Missing 'Z Upper' files"); }

    zfront_files.sort_by(|a, b| {a.file_name().cmp(&b.file_name())});
    zplane_files.sort_by(|a, b| {a.file_name().cmp(&b.file_name())});
    zrear_files.sort_by(|a, b| {a.file_name().cmp(&b.file_name())});
    zupper_files.sort_by(|a, b| {a.file_name().cmp(&b.file_name())});

    let mut z_front = vec![0_f32; size as usize * size as usize];
    let mut z_plane = vec![0_f32; size as usize * size as usize];
    let mut z_rear = vec![0_f32; size as usize * size as usize];
    let mut z_upper = vec![0_f32; size as usize * size as usize];

    for frame in 0..num_frames {
        let path_out = zmask_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out.exists() {
            continue;
        }

        let f_zplane = &zplane_files[frame];
        let f_zfront = &zfront_files[frame];
        let f_zrear = &zrear_files[frame];
        let f_zupper = &zupper_files[frame];

        read_depth_exr(&f_zfront.path(), &mut z_front);
        read_depth_exr(&f_zplane.path(), &mut z_plane);
        read_depth_exr(&f_zrear.path(), &mut z_rear);
        read_depth_exr(&f_zupper.path(), &mut z_upper);

        let zmask = depth_mask(frame, &z_front, &z_rear, &z_upper, &z_plane, size as u64);

        save_webp(path_out, size, &zmask);
    }

}