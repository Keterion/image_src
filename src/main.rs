use std::path::{Path, PathBuf};

use image::{self, DynamicImage, GenericImageView, Pixel, Rgb, RgbImage};

use log::{debug, info, trace};

/// contains a vector of rgb values and the amount of pixels
pub struct MinMaxImg {
    pixels: usize,
    rgb: Vec<[u8; 3]>,
}
impl MinMaxImg {
    pub fn new(pixels: usize, variant: Differential) -> Self {
        let rgb = match variant {
            Differential::Min => vec![[255, 255, 255]; pixels],
            Differential::Max => vec![[0, 0, 0]; pixels],
        };
        MinMaxImg { pixels, rgb }
    }
    pub fn min(&mut self, pxl: Rgb<u8>, index: usize) {
        if pxl.channels()[0] < self.rgb[index][0] {
            self.rgb[index][0] = pxl.channels()[0];
        }
        if pxl.channels()[1] < self.rgb[index][1] {
            self.rgb[index][1] = pxl.channels()[1];
        }
        if pxl.channels()[2] < self.rgb[index][2] {
            self.rgb[index][2] = pxl.channels()[2];
        }
    }
    pub fn max(&mut self, pxl: Rgb<u8>, index: usize) {
        if pxl.channels()[0] > self.rgb[index][0] {
            self.rgb[index][0] = pxl.channels()[0];
        }
        if pxl.channels()[1] > self.rgb[index][1] {
            self.rgb[index][1] = pxl.channels()[1];
        }
        if pxl.channels()[2] > self.rgb[index][2] {
            self.rgb[index][2] = pxl.channels()[2];
        }
    }
    pub fn difference(&self, pxl: Rgb<u8>, index: usize) -> [u8; 3] {
        [
            self.rgb[index][0].abs_diff(pxl.channels()[0]),
            self.rgb[index][1].abs_diff(pxl.channels()[1]),
            self.rgb[index][2].abs_diff(pxl.channels()[2]),
        ]
    }
    pub fn save(&self, path: &str, width: u32, height: u32) {
        if (width * height) as usize == self.pixels {
            let mut img = RgbImage::new(width, height);
            for (i, pixel) in img.pixels_mut().enumerate() {
                pixel.0 = self.rgb[i].into();
            }
            img.save(path).unwrap();
        } else {
            eprintln!("Unable to save image, width and height not fitting for pixel ammount");
        }
    }
}
/// Selection of min or max image
pub enum Differential {
    Min,
    Max,
}

fn main() {
    env_logger::init();
    let args: Vec<_> = std::env::args().collect();
    info!("Got arguments: {:#?}", args);
    let mut files: Vec<PathBuf> = Vec::new();
    if args.len() < 3 {
        eprintln!("Expected at least 2 arguments: encode/decode, file(s)");
        std::process::exit(1);
    }
    let should_encode: bool = if args[1] == "e" || args[1] == "encode" {
        info!("Encoding");
        true
    } else if args[1] == "d" || args[1] == "decode" {
        info!("Decoding");
        false
    } else {
        eprintln!("Wrong setting for en/decode. Please use either e, encode, d or decode");
        std::process::exit(1);
    };

    for arg in &args[2..] {
        let p = Path::new(&arg);
        if p.is_dir() {
            for file in get_files_from_folder(&p) {
                files.push(file);
            }
        } else {
            files.push(p.to_path_buf());
        }
    }

    info!("Files are:\n{:#?}", files);

    if should_encode {
        let (max_width, max_height) = get_largest_image(&files);
        debug!("Max width: {}\nMax height: {}", max_width, max_height);
        let max_pixels = max_height * max_width;
        let (min, max) = get_minmax(&files, max_pixels);

        min.save("min.png", max_width as u32, max_height as u32);
        max.save("max.png", max_width as u32, max_height as u32);
        encode(&min, &max, &files);
    } else {
        let min: Vec<[u8; 3]> = image::open("min.png")
            .unwrap()
            .pixels()
            .map(|pixel| {
                [
                    pixel.2.channels()[0],
                    pixel.2.channels()[1],
                    pixel.2.channels()[2],
                ]
            })
            .collect();
        let max: Vec<[u8; 3]> = image::open("max.png")
            .unwrap()
            .pixels()
            .map(|pixel| {
                [
                    pixel.2.channels()[0],
                    pixel.2.channels()[1],
                    pixel.2.channels()[2],
                ]
            })
            .collect();
        decode(&min, &max, &files);
    }

    for file in files {
        println!("{}", file.display());
    }
}
// important: when en/decoding write the switch value and switch afterwards
/// Encodes images using the provided min and max images
pub fn encode(min: &MinMaxImg, max: &MinMaxImg, images: &Vec<PathBuf>) {
    let mut use_max = [false; 3];
    let mut img: DynamicImage;
    let mut d_min: [u8; 3];
    let mut d_max: [u8; 3];
    for image_path in images {
        info!("Encoding {}...", image_path.display());
        img = image::open(image_path).expect("Unable to open image");
        let mut new = RgbImage::new(img.width(), img.height());
        for (i, (pixel, new_pixel)) in img.pixels().zip(new.pixels_mut()).enumerate() {
            //debug!("Next pixel");
            d_min = min.difference(pixel.2.to_rgb(), i);
            d_max = max.difference(pixel.2.to_rgb(), i);
            for j in 0..3 {
                process_color_encode(
                    &mut use_max[j],
                    min.rgb[i][j],
                    max.rgb[i][j],
                    (d_min[j], d_max[j]),
                    &mut new_pixel.channels_mut()[j],
                );
            }
        }
        debug!("Saving {}", image_path.display());
        // images need to be saved without lossy compression
        new.save(Path::new("encoded").join(Path::new(&format!(
            "{}.png",
            image_path.file_stem().unwrap().to_str().unwrap()
        ))))
        .unwrap();
        debug!("Saved image");
    }
}
/// decodes vector of images using the min and max "image" pixel vectors
pub fn decode(min: &Vec<[u8; 3]>, max: &Vec<[u8; 3]>, images: &Vec<PathBuf>) {
    let mut use_max: [bool; 3];
    let mut img: DynamicImage;
    for image_path in images {
        use_max = [false; 3]; // reset for each image
        info!("Decoding {}...", image_path.display());
        println!("working on decoding {}", image_path.display());
        img = image::open(image_path).expect("Unable to open image");
        let mut new = RgbImage::new(img.width(), img.height());
        for (i, (pixel, new_pixel)) in img.pixels().zip(new.pixels_mut()).enumerate() {
            //debug!("Next pixel");
            for j in 0..3 {
                process_color_decode(
                    &mut use_max[j],
                    min[i][j],
                    max[i][j],
                    pixel.2.to_rgb().channels()[j],
                    &mut new_pixel.channels_mut()[j],
                );
            }
        }
        new.save(Path::new("decoded").join(Path::new(image_path.file_name().unwrap())))
            .unwrap();
        debug!("Saved image");
    }
}

/// sets the pixel of the encoded image and switches r, g, b on whether to use min or max
pub fn process_color_encode(use_max: &mut bool, min: u8, max: u8, color: (u8, u8), pxl: &mut u8) {
    let middle = (max - min) / 2;
    //debug!("Middle: {}", middle);
    if *use_max {
        *pxl = color.1;
        if color.1 > middle {
            // switch if we're going to the other side of the middle
            *use_max = false;
        }
    } else {
        *pxl = color.0;
        if color.0 > middle {
            // switch if we're going to the other side of the middle
            *use_max = true;
        }
    }
}
/// reconstructs the image from the difference and min/max images
pub fn process_color_decode(use_max: &mut bool, min: u8, max: u8, difference: u8, pxl: &mut u8) {
    let middle = (max - min) / 2; // optimize
    let new: u8;
    if *use_max {
        new = if let Some(res) = max.checked_sub(difference) {
            res
        } else {
            trace!(
                "Subtraction overflow with values:\n max: {} - difference: {}",
                max,
                difference
            );
            0
        };
    } else {
        new = if let Some(res) = min.checked_add(difference) {
            res
        } else {
            trace!(
                "Addition overflow with values:\nmin: {} + difference: {}",
                min,
                difference
            );
            255
        };
    }
    *pxl = new;
    if difference > middle {
        trace!(
            "Difference {} larger than {}, switching from {}",
            difference,
            middle,
            use_max
        );
        *use_max = !*use_max;
    }
}

/// Get largest width and height from a vector of images
pub fn get_largest_image(images: &Vec<PathBuf>) -> (usize, usize) {
    let mut max_x = 0;
    let mut max_y = 0;
    for image in images {
        let img = image::open(image).expect(&format!("Unable to open image {}", image.display()));
        let w = img.width() as usize;
        let h = img.height() as usize;
        if w > max_x {
            max_x = w;
        }
        if h > max_y {
            max_y = h;
        }
    }
    (max_x, max_y)
}

/// Get minimum and maximum images from a given vector of images
pub fn get_minmax(images: &Vec<PathBuf>, max_pixels: usize) -> (MinMaxImg, MinMaxImg) {
    let mut min = MinMaxImg::new(max_pixels, Differential::Min);
    let mut max = MinMaxImg::new(max_pixels, Differential::Max);

    for image in images {
        let img = image::open(image).unwrap();
        for (i, pixel) in img.pixels().enumerate() {
            min.min(pixel.2.to_rgb(), i);
            max.max(pixel.2.to_rgb(), i);
        }
    }

    (min, max)
}

/// recursively get all files from a folder
pub fn get_files_from_folder(folder: &Path) -> Vec<PathBuf> {
    let mut res: Vec<PathBuf> = Vec::new();
    for file in std::fs::read_dir(folder).unwrap() {
        let f = file.unwrap().path();
        if f.is_dir() {
            res.append(&mut get_files_from_folder(&f))
        } else {
            res.push(f);
        }
    }
    res
}
