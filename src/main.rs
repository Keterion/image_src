use std::path::{Path, PathBuf};

use image::{self, GenericImageView, Pixel, Rgb, RgbImage};

struct MinMaxImg {
    pixels: usize,
    rgb: Vec<(u8, u8, u8)>,
}
impl MinMaxImg {
    fn new(pixels: usize, fill: (u8, u8, u8)) -> Self {
        let rgb: Vec<(u8, u8, u8)> = vec![fill; pixels];
        MinMaxImg {
            pixels,
            rgb,
        }
    }
    fn min(&mut self, pxl: Rgb<u8>, index: usize) {
        if pxl.channels()[0] < self.rgb[index].0 {
            self.rgb[index].0 = pxl.channels()[0];
        }
        if pxl.channels()[1] < self.rgb[index].1 {
            self.rgb[index].1 = pxl.channels()[1];
        }
        if pxl.channels()[2] < self.rgb[index].2 {
            self.rgb[index].2 = pxl.channels()[2];
        }
    }
    fn max(&mut self, pxl: Rgb<u8>, index: usize) {
        if pxl.channels()[0] > self.rgb[index].0 {
            self.rgb[index].0 = pxl.channels()[0];
        }
        if pxl.channels()[1] > self.rgb[index].1 {
            self.rgb[index].1 = pxl.channels()[1];
        }
        if pxl.channels()[2] > self.rgb[index].2 {
            self.rgb[index].2 = pxl.channels()[2];
        }
    }
    fn save(&self, path: &str, width: u32, height: u32) {
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

fn main() {
    let args = std::env::args();
    let mut files: Vec<PathBuf> = Vec::new();
    for arg in args.skip(1) {
        let p = Path::new(&arg);
        if p.is_dir() {
            for file in get_files_from_folder(&p) {
                files.push(file);
            }
        } else {
            files.push(p.to_path_buf());
        }
    }

    let (max_width, max_height) = get_largest_image(&files);
    let max_pixels = max_height * max_width;
    let (min, max) = get_minmax(&files, max_pixels);

    min.save("min.png", max_width as u32, max_height as u32);
    max.save("max.png", max_width as u32, max_height as u32);

    for file in files {
        println!("{}", file.display());
    }
}

fn get_largest_image(images: &Vec<PathBuf>) -> (usize, usize) {
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

fn get_minmax(images: &Vec<PathBuf>, max_pixels: usize) -> (MinMaxImg, MinMaxImg) {
    let mut min = MinMaxImg::new(max_pixels, (255, 255, 255));
    let mut max = MinMaxImg::new(max_pixels, (0, 0, 0));
    
    for image in images {
        let img = image::open(image).unwrap();
        for (i, pixel) in img.pixels().enumerate() {
            min.min(pixel.2.to_rgb(), i);
            max.max(pixel.2.to_rgb(), i);
        }
    }

    (min, max)
}

fn get_files_from_folder(folder: &Path) -> Vec<PathBuf> { // recursively gets all files from a
                                                          // folder
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
