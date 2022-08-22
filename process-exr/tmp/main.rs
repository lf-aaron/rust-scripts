use clap::Parser;
// use image::codecs::png;
use image::{EncodableLayout, ImageEncoder};
// use image::codecs::png::{CompressionType, FilterType};
// use image::ColorType::Rgb8;
use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::fs;
use std::path::{Path, PathBuf};
use exr::prelude::*;
use exr::meta::header::{Header};
use webp;


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {

    #[clap(long, parse(from_os_str))]
    front: PathBuf,

    #[clap(long, parse(from_os_str))]
    rear: PathBuf,

    #[clap(long, parse(from_os_str))]
    upper: PathBuf,

    #[clap(long, parse(from_os_str))]
    zmask: PathBuf,

    #[clap(long, parse(from_os_str))]
    matte: PathBuf,

    #[clap(long, parse(from_os_str))]
    index: PathBuf,
}

// Used for picking channel from zmask
// Returns index of channel with largest value, or -1 if no values are above a threshold.
fn mask(x: &[u8]) -> i32 {
    let threshold = 5;
    if x[0] > threshold { return 0; }
    if x[1] > threshold { return 1; }
    if x[2] > threshold { return 2; }
    return -1;
}

fn get_asset_map() -> HashMap<[u8; 4], u8> {
    let array: [(u8, f32); 32] = [
        (0, 0.0),
        // (0, -1.1562982805507717e+33), // Mag shell
        // (0, -2.7688418327562324e-28), // Background
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

    HashMap::from(array.map(|v | { (v.1.to_be_bytes(), v.0) }))

}

fn get_channels(header: &Header, layers: [&str; 4]) -> Vec<String>{
    let mut sorted = Vec::<String>::new();
    for layer in layers {
        for channel in &header.channels.list {
            let name = channel.name.to_string();
            if name.as_str().contains(layer) {
                sorted.push(name);
            }
        }
    }
    return sorted;
}


fn read_cryptomatte_exr(path: &Path) -> (Vec2<usize>, Vec<f32>) {
    
    let index_layers = [
        "CryptoAsset00.R",
        "CryptoAsset00.B",
        "CryptoAsset01.R",
        "CryptoAsset01.B"
    ];

    let matte_layers = [
        "CryptoAsset00.G",
        "CryptoAsset00.A",
        "CryptoAsset01.G",
        "CryptoAsset01.A"
    ];
    
    let bytes = BufReader::new(fs::File::open(path).unwrap());
    let exr_reader = exr::block::read(bytes, false).unwrap();
    let header = exr_reader.headers().first().unwrap();
    
    let window = header.shared_attributes.display_window;
    let data_window_offset = header.own_attributes.layer_position - window.position;
    let width = window.size.width();
    let height = window.size.height();
    
    let index_channels = get_channels(header, index_layers);
    let matte_channels = get_channels(header, matte_layers);
    let channel_count = index_channels.len() + matte_channels.len();

    let image_pixels = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .specific_channels()
        .required(index_channels[0].as_str())
        .required(index_channels[1].as_str())
        .required(index_channels[2].as_str())
        .required(index_channels[3].as_str())
        .required(matte_channels[0].as_str())
        .required(matte_channels[1].as_str())
        .required(matte_channels[2].as_str())
        .required(matte_channels[3].as_str())
        .collect_pixels(
            move |_size, _channels| vec![0_f32; window.size.area() * channel_count],
            move |buffer, window_index, (i1, i2, i3, i4, m1, m2, m3, m4): (f32, f32, f32, f32, f32, f32, f32, f32)| {
                // Copied from image-rs openexr implementation
                let index = window_index.to_i32() + data_window_offset;
                if index.x() >= 0 && index.y() >= 0  && index.x() < width as i32 && index.y() < height as i32 {
                    let index = index.to_usize("index bug").unwrap();
                    let first_f32_index = index.flat_index_for_size(window.size);
                    buffer[first_f32_index * channel_count..(first_f32_index + 1) * channel_count]
                        .copy_from_slice(&[i1, i2, i3, i4, m1, m2, m3, m4][0..channel_count]);
                }
            },
        )
        .first_valid_layer()
        .all_attributes()
        .from_chunks(exr_reader)
        .unwrap()
        .layer_data
        .channel_data
        .pixels;
    
    return (window.size, image_pixels);
}

fn composite_cryptomatte(map: &HashMap<[u8; 4], u8>, size: Vec2<usize>, zmask: &Vec<u8>, exr1: &Vec<f32>, exr2: &Vec<f32>, exr3: &Vec<f32>) -> (Vec<u8>, Vec<u8>) {
    let num_pixels = size.area();
    let mut index = vec![0_u8; (num_pixels * 3) as usize];
    let mut matte = vec![0_u8; (num_pixels * 3) as usize];
    for i in 0..num_pixels {
        let z = &zmask[3*i..3*i+3];

        let idx1 = &exr1[8*i..8*i+4];
        let idx2 = &exr2[8*i..8*i+4];
        let idx3 = &exr3[8*i..8*i+4];

        let val1 = &exr1[8*i+4..8*i+8];
        let val2 = &exr2[8*i+4..8*i+8];
        let val3 = &exr3[8*i+4..8*i+8];

        let (idx, val) = match mask(z) {
            0 => (idx1, val1),
            1 => (idx2, val2),
            2 => (idx3, val3),
            _ => ([0_f32; 4].as_slice(), [0_f32; 4].as_slice())
        };

        // let id = idx.into_iter().map(|x| { map[&x.to_be_bytes()] }).collect::<Vec<u8>>();
        let id = idx.into_iter().map(|x| {
            let y = match map.get(&x.to_be_bytes()) {
                None => panic!("Missing key: {}", *x),
                Some(num) => *num
            };
            return y;
        }).collect::<Vec<u8>>();
        let i_r = id[0] | (id[1] & 0b11) << 6;
        let i_g = (id[1] & 0b111100) >> 2 | (id[2] & 0b1111) << 4;
        let i_b = (id[2] & 0b110000) >> 4 | id[3] << 2;

        let m_r = (2.0 * val[1] * 255_f32) as u8;
        let m_g = (2.0 * val[2] * 255_f32) as u8;
        let m_b = (2.0 * val[3] * 255_f32) as u8;

        index.splice(3*i..3*i+3, [i_r, i_g, i_b]);
        matte.splice(3*i..3*i+3, [m_r, m_g, m_b]);
    }
    return (index, matte);
}


fn save_webp(path: PathBuf, size: Vec2<usize>, pixels: &Vec<u8>) {
    // print!("path: {:?}\n", path);
    let w = size.width() as u32;
    let h = size.height() as u32;
    let img = webp::Encoder::from_rgb(pixels, w as u32, h as u32).encode_lossless();
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}


// fn save_png(path: PathBuf, size: Vec2<usize>, pixels: &Vec<u8>) {
//     let buffered_file_write = &mut BufWriter::new(fs::File::create(path).unwrap());
//     png::PngEncoder::new_with_quality(
//         buffered_file_write,
//         CompressionType::Best,
//         FilterType::NoFilter
//     )
//     .write_image(
//         pixels,
//         size.width() as u32,
//         size.height() as u32,
//         Rgb8
//     )
//     .unwrap();
// }


fn main() {
    let args = CliArgs::parse();
    
    let front_dir = &args.front;
    let rear_dir = &args.rear;
    let upper_dir = &args.upper;
    let zmask_dir = &args.zmask;
    let matte_dir = &args.matte;
    let index_dir = &args.index;

    let z_files = fs::read_dir(zmask_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let front_files = fs::read_dir(front_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let rear_files = fs::read_dir(rear_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();
    let upper_files = fs::read_dir(upper_dir).unwrap().map(|f| f.unwrap()).collect::<Vec<fs::DirEntry>>();

    // print!("Z: {:?}\n", z_files.len());
    // print!("Front: {:?}\n", front_files.len());
    // print!("Rear: {:?}\n", rear_files.len());
    // print!("Upper {:?}\n", upper_files.len());

    let map = get_asset_map();

    for i in 0..z_files.len() {
        let f_z = &z_files[i];
        let f_front = &front_files[i];
        let f_rear = &rear_files[i];
        let f_upper = &upper_files[i];

        let z_mask = image::open(f_z.path()).unwrap().to_rgb8().as_bytes().to_vec();
        let (size_front, exr1) = read_cryptomatte_exr(&f_front.path());
        let (size_rear, exr2) = read_cryptomatte_exr(&f_rear.path());
        let (size_upper, exr3) = read_cryptomatte_exr(&f_upper.path());

        if !(Vec2::eq(&size_front, &size_rear) && Vec2::eq(&size_rear, &size_upper)) {
            panic!("EXR size mismatch");
        }
        let size = size_rear;

        let (index, matte) = composite_cryptomatte(&map, size, &z_mask, &exr1, &exr2, &exr3);
        save_webp(index_dir.join(f_z.file_name()), size, &index);
        save_webp(matte_dir.join(f_z.file_name()), size, &matte);
    }
}
