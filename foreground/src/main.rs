use arrayfire::*;
use clap::Parser;
use image::{EncodableLayout};
use std::io::{BufWriter, Write};
use std::fs;
use std::path::{Path, PathBuf};
use exr::prelude::*;
use webp;


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

    #[clap(long)]
    device: i32,

    #[clap(long)]
    overwrite: bool,
}


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
            0 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::B),        // AO.B
            1 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::G),        // AO.G
            2 => obj.set_channel(f(&channels[i]), ForegroundPass::AO, RGBAChannel::R),        // AO.R
            // 3                                                                              // Combined.A
            // 4                                                                              // Combined.B
            // 5                                                                              // Combined.G
            // 6                                                                              // Combined.R
            // 7                                                                              // Crypto00.A
            // 8                                                                              // Crypto00.B
            // 9                                                                              // Crypto00.G
            // 10                                                                             // Crypto00.R
            // 11                                                                             // Crypto01.A
            // 12                                                                             // Crypto01.B
            // 13                                                                             // Crypto01.G
            // 14                                                                             // Crypto01.R
            15 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::B),  // Diffuse.B
            16 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::G),  // Diffuse.G
            17 => obj.set_channel(f(&channels[i]), ForegroundPass::DIFFUSE, RGBAChannel::R),  // Diffuse.R
            18 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::B),   // Glossy.B
            19 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::G),   // Glossy.G
            20 => obj.set_channel(f(&channels[i]), ForegroundPass::GLOSSY, RGBAChannel::R),   // Glossy.R
            _ => {},
        };
    }

    return obj;
}


fn composite(
    front: ForegroundStruct,
    rear: ForegroundStruct,
    upper: ForegroundStruct,
    zmask: &Vec<u8>,
    size: u64,
) -> Vec<u8> {
    
    let dims = dim4!(size, size, 3);

    let mut light = vec!(0; dims.elements() as usize);

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

    return light;
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
    let device = args.device;
    let overwrite = args.overwrite;

    set_backend(Backend::CUDA);
    set_device(device);

    let front_files = fs::read_dir(front_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let rear_files = fs::read_dir(rear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let upper_files = fs::read_dir(upper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let zmask_files = fs::read_dir(zmask_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    let num_frames = 144;
    if front_files.len() != num_frames { panic!("Missing 'Front' files"); }
    if rear_files.len() != num_frames { panic!("Missing 'Rear' files"); }
    if upper_files.len() != num_frames { panic!("Missing 'Upper' files"); }
    if zmask_files.len() != num_frames { panic!("Missing 'ZMask' files"); }

    for frame in 0..num_frames {
        let path_out = light_dir.join(format!("{:0>4}", (121 + frame).to_string())).with_extension("webp");
        if !overwrite && path_out.exists() {
            continue;
        }

        let f_front = &front_files[frame];
        let f_rear = &rear_files[frame];
        let f_upper = &upper_files[frame];
        let f_zmask = &zmask_files[frame];

        let front = read_foreground_exr(&f_front.path(), size);
        let rear = read_foreground_exr(&f_rear.path(), size);
        let upper = read_foreground_exr(&f_upper.path(), size);
        let zmask = image::open(f_zmask.path()).unwrap().to_rgb8().as_bytes().to_vec();

        let light = composite(
            front,
            rear,
            upper,
            &zmask,
            size as u64,
        );

        save_webp(path_out, size, &light, WebpCompressionType::LOSSY(100.0));
    }
}
