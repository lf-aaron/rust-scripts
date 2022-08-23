// use clap::Parser;
// // use image::codecs::png;
// // use image::codecs::png::{CompressionType, FilterType};
// // use image::ColorType::Rgb8;
// // use image::ImageEncoder;
// // use rayon::iter::ParallelIterator;
// // use rayon::prelude::ParallelBridge;
// use std::collections::HashMap;
// use std::io::{BufReader, BufWriter, Write};
// use std::fs;
// use std::fs::File;
// // use std::os::windows::process;
// use std::path::{Path, PathBuf};
// use exr::prelude::*;
// use exr::block::reader::Reader;
// use webp;


// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = None)]
// struct CliArgs {
//     /// Name of input file
//     #[clap(parse(from_os_str))]
//     in_file: PathBuf,

//     /// Name of output file
//     #[clap(short = 'o', parse(from_os_str))]
//     out_file: PathBuf,
// }


// fn get_material_map(filename: &str) -> HashMap<[u8; 4], u8> {
//     let raw_json = fs::read_to_string(filename).expect(format!("Failed to read '{}'", filename).as_str());

//     serde_json::from_str::<serde_json::Value>(raw_json.as_str()).unwrap().as_object().unwrap().values().map(|v| {
//         let mat = v.as_object().unwrap();
//         let hash = (mat["hash"].as_f64().unwrap() as f32).to_be_bytes();
//         let id = mat["id"].as_i64().unwrap() as u8;
//         (hash, id)
//     }).collect::<HashMap<_, _>>()
// }

// fn get_asset_map() -> HashMap<[u8; 4], u8> {
//     let array: [(u8, f32); 31] = [
//         (0, 0.0),
//         (1, -0.03498752787709236),
//         (1, 0.021271370351314545),
//         (2, -7.442164937651292e-35),
//         (3, -6.816108887753408e+29),
//         (4, 0.00035458870115689933),
//         (5, -2.1174496448267268e-37),
//         (6, 1.4020313126302311e+32),
//         (7, -1.0356748461253123e-29),
//         (8, -2.9085143335341026e+36),
//         (9, 1.3880169547064725e-07),
//         (10, -1.259480075076364e+31),
//         (11, 9.950111644430328e-20),
//         (12, 7.755555963998422e+23),
//         (13, 7.694573644696632e-19),
//         (14, -5.1650727722774545e-23),
//         (15, 9.80960464477539),
//         (16, -2.863075394543557e-07),
//         (17, -1.1106028290273499e+26),
//         (18, 5.081761389253177e+22),
//         (19, -6.4202393950590105e+25),
//         (20, -4.099688753251169e+19),
//         (21, -4.738090716008833e+34),
//         (22, 1.3174184410047474e-08),
//         (23, -0.014175964519381523),
//         (24, 2.4984514311654493e-05),
//         (25, -8.232201253122184e-06),
//         (26, 1.2103820479584479e-20),
//         (27, -2.508242528606597e-12),
//         (28, 1.5731503249895985e+26),
//         (29, 1.4262572893553038e-11),
//         (30, -84473296.0),
//     ];

//     HashMap::from(array.map(|v | { (v.1.to_be_bytes(), v.0) }))

// }


// // fn read_cryptomatte_exr(path: &Path, layers: [&str; 4]) -> (usize, usize, Vec<f32>) {
// //     let in_bytes = BufReader::new(fs::File::open(path).unwrap());
// //     let exr_reader = exr::block::read(in_bytes, false).unwrap();
// //     let header = exr_reader.headers().first().unwrap();

// //     let window = header.shared_attributes.display_window;
// //     let data_window_offset = header.own_attributes.layer_position - window.position;
// //     let width = window.size.width();
// //     let height = window.size.height();

// //     let mut set = Vec::<String>::new();
// //     for ch in &header.channels.list {
// //         set.push(ch.name.to_string());
// //     }

// //     let channels = layers.into_iter().map(|x| {
// //         (&set).into_iter().find(|ch| ch.contains(x)).unwrap().as_str()
// //     }).collect::<Vec<&str>>();

// //     let channel_count = channels.len();
// //     let image_pixels = exr::prelude::read()
// //         .no_deep_data()
// //         .largest_resolution_level()
// //         .specific_channels()
// //         .required(channels[0])
// //         .required(channels[1])
// //         .required(channels[2])
// //         .required(channels[3])
// //         .collect_pixels(
// //             move |_size, _channels| vec![0_f32; window.size.area() * channel_count],
// //             move |buffer, index_in_data_window, (r, g, b, a): (f32, f32, f32, f32)| {
// //                 // Copied from image-rs openexr implementation
// //                 let index = index_in_data_window.to_i32() + data_window_offset;
// //                 if index.x() >= 0
// //                     && index.y() >= 0
// //                     && index.x() < window.size.width() as i32
// //                     && index.y() < window.size.height() as i32
// //                 {
// //                     let index = index.to_usize("index bug").unwrap();
// //                     let first_f32_index = index.flat_index_for_size(window.size);

// //                     buffer[first_f32_index * channel_count..(first_f32_index + 1) * channel_count]
// //                         .copy_from_slice(&[r, g, b, a][0..channel_count]);
// //                 }
// //             },
// //         )
// //         .first_valid_layer()
// //         .all_attributes()
// //         .from_chunks(exr_reader)
// //         .unwrap()
// //         .layer_data
// //         .channel_data
// //         .pixels;
    
// //     (width, height, image_pixels)
// // }

// fn read_pixels(reader: Reader<BufReader<File>>, layers: [&str;  4]) {
//     let header = reader.headers().first().unwrap();
//     let window = header.shared_attributes.display_window;
//     let width = window.size.width();
//     let height = window.size.height();
//     let data_window_offset = header.own_attributes.layer_position - window.position;

//     let mut set = Vec::<String>::new();
//     for ch in &header.channels.list {
//         set.push(ch.name.to_string());
//     }

//     let channels = layers.into_iter().map(|x| {
//         (&set).into_iter().find(|ch| ch.contains(x)).unwrap().as_str()
//     }).collect::<Vec<&str>>();

//     let channel_count = channels.len();
//     let image_pixels = exr::prelude::read()
//         .no_deep_data()
//         .largest_resolution_level()
//         .specific_channels()
//         .required(channels[0])
//         .required(channels[1])
//         .required(channels[2])
//         .required(channels[3])
//         .collect_pixels(
//             move |_size, _channels| vec![0_f32; window.size.area() * channel_count],
//             move |buffer, index_in_data_window, (r, g, b, a): (f32, f32, f32, f32)| {
//                 // Copied from image-rs openexr implementation
//                 let index = index_in_data_window.to_i32() + data_window_offset;
//                 if index.x() >= 0
//                     && index.y() >= 0
//                     && index.x() < window.size.width() as i32
//                     && index.y() < window.size.height() as i32
//                 {
//                     let index = index.to_usize("index bug").unwrap();
//                     let first_f32_index = index.flat_index_for_size(window.size);

//                     buffer[first_f32_index * channel_count..(first_f32_index + 1) * channel_count]
//                         .copy_from_slice(&[r, g, b, a][0..channel_count]);
//                 }
//             },
//         )
//         .first_valid_layer()
//         .all_attributes()
//         .from_chunks(reader)
//         .unwrap()
//         .layer_data
//         .channel_data
//         .pixels;
// }

// fn read_cryptomatte_exr(path: &Path) -> (usize, usize, Vec<f32>, Vec<f32>) {
    
//     let id_layers = [
//         "CryptoAsset00.R",
//         "CryptoAsset00.B",
//         "CryptoAsset01.R",
//         "CryptoAsset01.B"
//     ];

//     let matte_layers = [
//         "CryptoAsset00.G",
//         "CryptoAsset00.A",
//         "CryptoAsset01.G",
//         "CryptoAsset01.A"
//     ];

//     let in_bytes = BufReader::new(fs::File::open(path).unwrap());
//     let exr_reader = exr::block::read(in_bytes, false).unwrap();
//     let header = exr_reader.headers().first().unwrap();

//     let window = header.shared_attributes.display_window;
//     let data_window_offset = header.own_attributes.layer_position - window.position;
//     let width = window.size.width();
//     let height = window.size.height();

//     let mut set = Vec::<String>::new();
//     for ch in &header.channels.list {
//         set.push(ch.name.to_string());
//     }

//     let channels = id_layers.into_iter().map(|x| {
//         (&set).into_iter().find(|ch| ch.contains(x)).unwrap().as_str()
//     }).collect::<Vec<&str>>();

//     let channel_count = channels.len();
//     let image_pixels = exr::prelude::read()
//         .no_deep_data()
//         .largest_resolution_level()
//         .specific_channels()
//         .required(channels[0])
//         .required(channels[1])
//         .required(channels[2])
//         .required(channels[3])
//         .collect_pixels(
//             move |_size, _channels| vec![0_f32; window.size.area() * channel_count],
//             move |buffer, index_in_data_window, (r, g, b, a): (f32, f32, f32, f32)| {
//                 // Copied from image-rs openexr implementation
//                 let index = index_in_data_window.to_i32() + data_window_offset;
//                 if index.x() >= 0
//                     && index.y() >= 0
//                     && index.x() < window.size.width() as i32
//                     && index.y() < window.size.height() as i32
//                 {
//                     let index = index.to_usize("index bug").unwrap();
//                     let first_f32_index = index.flat_index_for_size(window.size);

//                     buffer[first_f32_index * channel_count..(first_f32_index + 1) * channel_count]
//                         .copy_from_slice(&[r, g, b, a][0..channel_count]);
//                 }
//             },
//         )
//         .first_valid_layer()
//         .all_attributes()
//         .from_chunks(exr_reader)
//         .unwrap()
//         .layer_data
//         .channel_data
//         .pixels;
    
//     (width, height, image_pixels)
// }

// // fn process_cryptomatte_exr(path_in: &Path, path_out: &Path, map: &HashMap<[u8; 4], u8>) {

// //     let (width, height, pixels) = read_cryptomatte_exr(path_in);
// //     let mut output_image = vec![0_u8; (width * height * 3) as usize];

// //     pixels.chunks(4).into_iter().enumerate().for_each(|(i, val)| {
// //         let id = val.into_iter().map(|x| { map[&x.to_be_bytes()] }).collect::<Vec<u8>>();
// //         let r = id[0] | (id[1] & 0b11) << 6;
// //         let g = (id[1] & 0b111100) >> 2 | (id[2] & 0b1111) << 4;
// //         let b = (id[2] & 0b110000) >> 4 | id[3] << 2;
// //         output_image.splice(3*i..3*i+3, [r, g, b]);
// //     });

// //     let buffered_file_write = &mut BufWriter::new(fs::File::create(path_out).unwrap());
// //     png::PngEncoder::new_with_quality(
// //         buffered_file_write,
// //         CompressionType::Best,
// //         FilterType::NoFilter
// //     )
// //     .write_image(
// //         &output_image,
// //         width as u32,
// //         height as u32,
// //         Rgb8
// //     )
// //     .unwrap();

// // }

// fn process_cryptomatte_exr(path_in: &Path, path_out: &Path, map: &HashMap<[u8; 4], u8>) {

//     let (width, height, pixels) = read_cryptomatte_exr(path_in);
//     let mut output_image = vec![0_u8; (width * height * 3) as usize];

//     pixels.chunks(4).into_iter().enumerate().for_each(|(i, val)| {
//         let id = val.into_iter().map(|x| { map[&x.to_be_bytes()] }).collect::<Vec<u8>>();
//         let r = id[0] | (id[1] & 0b11) << 6;
//         let g = (id[1] & 0b111100) >> 2 | (id[2] & 0b1111) << 4;
//         let b = (id[2] & 0b110000) >> 4 | id[3] << 2;
//         output_image.splice(3*i..3*i+3, [r, g, b]);
//     });

//     let img = webp::Encoder::from_rgb(&output_image, width as u32, height as u32).encode_lossless();
//     let mut buffered_file_write = BufWriter::new(fs::File::create(path_out).unwrap());
//     buffered_file_write.write_all(&img).unwrap();

//     // let buffered_file_write = &mut BufWriter::new(fs::File::create(path_out).unwrap());
//     // png::PngEncoder::new_with_quality(
//     //     buffered_file_write,
//     //     CompressionType::Best,
//     //     FilterType::NoFilter
//     // )
//     // .write_image(
//     //     &output_image,
//     //     width as u32,
//     //     height as u32,
//     //     Rgb8
//     // )
//     // .unwrap();

// }


// fn main() {
//     let args = CliArgs::parse();
    
//     let path_in = &args.in_file;
//     let path_out = &args.out_file;

//     // let map = get_material_map("./material_map.json");
//     let map = get_asset_map();
    
//     process_cryptomatte_exr(path_in, path_out, &map);

//     // let files_to_process = fs::read_dir(&args.in_file)
//     //     .unwrap()
//     //     .map(|f| f.unwrap())
//     //     .filter(|f| f.path().extension().unwrap() == "exr");
        
//     // if fs::read_dir(&args.out_file).is_err() {
//     //     fs::create_dir(&args.out_file).unwrap();
//     // }
    
//     // println!("{:?}", map);
    
//     // files_to_process.par_bridge().for_each(|f| {
//     //     let path_in = &f.path();
//     //     let path_out = &args.out_file.join(Path::new(path_in.file_name().unwrap()).with_extension("png"));
//     //     process_cryptomatte_exr(path_in, path_out, &map)
//     // });
// }
