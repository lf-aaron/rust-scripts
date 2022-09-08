use image::{EncodableLayout};
use std::io::{BufWriter, Write};
use std::fs;
use std::path::{Path, PathBuf};
use exr::prelude::*;
use webp;

#[derive(Debug)]
pub enum RGBAChannel {
    R,
    G,
    B,
    A,
}

pub enum WebpCompressionType {
    LOSSY(f32),
    LOSSLESS,
}

pub fn save_webp(path: PathBuf, size: u32, pixels: &Vec<u8>, compression: WebpCompressionType) {
    let img = match compression {
        WebpCompressionType::LOSSLESS => webp::Encoder::from_rgb(pixels, size, size).encode_lossless(),
        WebpCompressionType::LOSSY(quality) => webp::Encoder::from_rgb(pixels, size, size).encode(quality),
    };
    let _ = fs::create_dir_all(path.clone().parent().unwrap());
    let mut buffered_file_write = BufWriter::new(fs::File::create(path).unwrap());
    buffered_file_write.write_all(&img).unwrap();
}
