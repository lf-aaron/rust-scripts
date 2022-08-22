use exr::prelude::f16;
use image::EncodableLayout;
use itertools::Itertools;
use std::error::Error;
use std::fs;

pub fn process_lut1d(filename: &str) -> Result<(), Box<dyn Error>> {
    let raw_file = fs::read_to_string(filename)?;
    let values = raw_file
        .lines()
        .skip(5)
        .take(4096)
        .map(|f| f.replace(" ", "").parse::<f16>().unwrap().to_bits())
        .collect_vec();

    fs::write(
        "./filmic_to_0-70_1-03.bin",
        values.as_bytes()
    )
    .unwrap();

    return Ok(());
}

pub fn process_lut3d(filename: &str) -> Result<(), Box<dyn Error>> {
    let raw_file = fs::read_to_string(filename)?;
    let lines = raw_file.lines().skip(3).collect_vec();
    let mut out_bytes = vec![f16::from_f32(0.0).to_bits(); 65_usize.pow(3) * 3];
    for line in lines {
        let mut parts = line.split(" ").into_iter();
        let in_r = parts.next().unwrap().parse::<usize>().unwrap();
        let in_g = parts.next().unwrap().parse::<usize>().unwrap();
        let in_b = parts.next().unwrap().parse::<usize>().unwrap();
        let out_r = parts.next().unwrap().parse::<f16>().unwrap();
        let out_g = parts.next().unwrap().parse::<f16>().unwrap();
        let out_b = parts.next().unwrap().parse::<f16>().unwrap();
        out_bytes[(in_r + in_g * 65 + in_b * 65_usize.pow(2)) * 3] = out_r.to_bits();
        out_bytes[(in_r + in_g * 65 + in_b * 65_usize.pow(2)) * 3 + 1] = out_g.to_bits();
        out_bytes[(in_r + in_g * 65 + in_b * 65_usize.pow(2)) * 3 + 2] = out_b.to_bits();
    }

    fs::write(
        "./filmic_desat65cube.bin",
        out_bytes.as_bytes()
    )
    .unwrap();

    return Ok(());
}
