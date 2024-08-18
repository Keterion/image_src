use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use image::{self, DynamicImage, GenericImageView, Pixel, Rgb, RgbImage};

use clap::*;
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

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[clap(trailing_var_arg = true)]
struct Arguments {
    /// Select between en or decoding images
    #[clap(value_enum)]
    command: EnOrDecode,
    /// Specify minimum and maximum image paths
    #[arg(long, short, num_args = 2)]
    specific_mm: Option<Vec<PathBuf>>,
    /// Output directory
    #[clap(long, short)]
    output_dir: Option<PathBuf>,
    /// paths to en or decode from
    #[clap(num_args=1..)]
    images: Vec<PathBuf>,
}

#[derive(clap::ValueEnum, Clone)]
enum EnOrDecode {
    Encode,
    Decode,
}

fn main() {
    env_logger::init();
    let args = Arguments::parse();
    let mut files: Vec<PathBuf> = Vec::new();
    if args.images.len() == 0 {
        eprintln!("Specify at least one image path");
        std::process::exit(1);
    }
    for path in args.images {
        if path.is_dir() {
            for subpath in get_files_from_folder(&path) {
                files.push(subpath);
            }
        } else {
            files.push(path);
        }
    }
    files.reverse();
    info!("Finished loading images");
    debug!("Images: {:#?}", files);
    match args.command {
        EnOrDecode::Encode => {
            let (min, max): (MinMaxImg, MinMaxImg);
            if args.specific_mm.is_none() {
                info!("Generating min and max images");
                let (max_width, max_height) = get_largest_image(&files);
                debug!("Max width: {}\nMax height: {}", max_width, max_height);
                let max_pixels = max_height * max_width;
                (min, max) = get_minmax(&files, max_pixels);
                min.save("min.png", max_width as u32, max_height as u32);
                max.save("max.png", max_width as u32, max_height as u32);
            } else {
                info!("Using specified min and max images");
                if let Some(specific_mm) = &args.specific_mm {
                    debug!("Min: {}, Max: {}", specific_mm[0].display(), specific_mm[1].display());
                    (min, max) = (
                        {
                            let img = image::open(&specific_mm[0]).unwrap();
                            MinMaxImg {
                                pixels: (img.width() * img.height()) as usize,
                                rgb: img_to_rgb(&specific_mm[0]),
                            }
                        },
                        {
                            let img = image::open(&specific_mm[1]).unwrap();
                            MinMaxImg {
                                pixels: (img.width() * img.height()) as usize,
                                rgb: img_to_rgb(&specific_mm[1]),
                            }
                        },
                    );
                } else {
                    eprintln!("Min and Max specified wrong.");
                    std::process::exit(1);
                }
            }
            info!("Encoding images...");
            encode(&min, &max, &files, args.output_dir.unwrap_or(PathBuf::from_str("encoded").unwrap()));
        }
        EnOrDecode::Decode => {
            let (min, max): (Vec<[u8; 3]>, Vec<[u8; 3]>);
            if let Some(specific_mm) = args.specific_mm {
                info!("Using specified min and max images");
                debug!("Min: {}, Max: {}", specific_mm[0].display(), specific_mm[1].display());
                min = img_to_rgb(&specific_mm[0]);
                max = img_to_rgb(&specific_mm[1]);
            } else {
                info!("Using default min and max images");
                debug!("Min: min.png, Max: max.png");
                min = img_to_rgb(&PathBuf::from_str("min.png").unwrap());
                max = img_to_rgb(&PathBuf::from_str("max.png").unwrap());
            }
            info!("Decoding images...");
            decode(&min, &max, &files, args.output_dir.unwrap_or(PathBuf::from_str("decoded").unwrap()));
        }
    }
}
/// Transform an image(path) into a vector of rgb values
pub fn img_to_rgb(image: &PathBuf) -> Vec<[u8; 3]> {
    let img = image::open(image).expect(&format!("Unable to open image at '{}'", image.display()));
    img.pixels()
        .map(|pxl| {
            [
                pxl.2.channels()[0],
                pxl.2.channels()[1],
                pxl.2.channels()[2],
            ]
        })
        .collect()
}
// important: when en/decoding write the switch value and switch afterwards
/// Encodes images using the provided min and max images
pub fn encode(min: &MinMaxImg, max: &MinMaxImg, images: &Vec<PathBuf>, output_dir: PathBuf) {
    let mut use_max: [bool; 3];
    let mut img: DynamicImage;
    let mut d_min: [u8; 3];
    let mut d_max: [u8; 3];
    for image_path in images {
        info!("Encoding {}...", image_path.display());
        use_max = [false; 3];
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
        new.save(output_dir.join(Path::new(&format!(
            "{}.png",
            image_path.file_stem().unwrap().to_str().unwrap()
        ))))
        .unwrap();
        debug!("Saved image");
    }
}
/// decodes vector of images using the min and max "image" pixel vectors
pub fn decode(min: &Vec<[u8; 3]>, max: &Vec<[u8; 3]>, images: &Vec<PathBuf>, output_dir: PathBuf) {
    let mut use_max: [bool; 3];
    let mut img: DynamicImage;
    for image_path in images {
        use_max = [false; 3]; // reset for each image
        info!("Decoding {}...", image_path.display());
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
        new.save(output_dir.join(Path::new(image_path.file_name().unwrap())))
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

/// Get largest image from a vector of images
pub fn get_largest_image(images: &Vec<PathBuf>) -> (usize, usize) {
    debug!("Getting largest image dimensions");
    let mut max_w = 0;
    let mut max_h = 0;
    let mut max_pixels = 0;
    for image in images {
        trace!("{}", image.display());
        let img = image::open(image).expect(&format!("Unable to open image {}", image.display()));
        let w = img.width() as usize;
        let h = img.height() as usize;
        if max_pixels < w * h {
            max_w = w;
            max_h = h;
            max_pixels = w * h;
        }
    }
    (max_w, max_h)
}

/// Get minimum and maximum images from a given vector of images
pub fn get_minmax(images: &Vec<PathBuf>, max_pixels: usize) -> (MinMaxImg, MinMaxImg) {
    debug!("Getting minimum and maximum");
    let mut min = MinMaxImg::new(max_pixels, Differential::Min);
    let mut max = MinMaxImg::new(max_pixels, Differential::Max);

    for image in images {
        trace!("{}", image.display());
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
