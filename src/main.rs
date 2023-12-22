use std::{
    env,
    fs::{self},
    io::{Write, Read},
    path::PathBuf,
};

use eframe::egui;
use egui_extras::RetainedImage;
use image::{GenericImageView};
use skia_safe::{AlphaType, Color4f, ColorType, EncodedImageFormat, ImageInfo, Paint, Rect, Surface};
use flate2::{write::GzEncoder, Compression};

static TEMP_RESULT_PATH: &str = "temp.png";

struct Sam;

impl Sam {
    fn convert_bytes_to_u32_ne(bytes: &[u8]) -> u32 {
        let mut result = [0u8; 4];
        result.copy_from_slice(bytes);
        u32::from_ne_bytes(result)
    }

    fn convert_png_to_hif(path: PathBuf) -> Result<(), std::io::Error> {
        let img = image::open(&path).expect("File not found!");
    
        let mut pixel_data: Vec<u8> = Vec::new();
    
        let width_bytes: [u8; 4] = img.width().to_ne_bytes();
        let height_bytes: [u8; 4] = img.height().to_ne_bytes();
    
        pixel_data.extend_from_slice(&width_bytes);
        pixel_data.extend_from_slice(&height_bytes);
    
        for pixel in img.pixels() {
            pixel_data.push(pixel.2[0]); // R
            pixel_data.push(pixel.2[1]); // G
            pixel_data.push(pixel.2[2]); // B
        }
    
        let path_to_hif = path.with_extension("hif");
    
        fs::write(path_to_hif, &pixel_data)?;
    
        Ok(())
    }    

    fn hif_to_png(path: PathBuf) -> (u32, u32) {
        let contents = fs::read(&path).expect("Couldn't read file.");
    
        let (width, height) = (
            Sam::convert_bytes_to_u32_ne(&contents[0..4]),
            Sam::convert_bytes_to_u32_ne(&contents[4..8]),
        );
    
        let pixel_data = &contents[8..];
    
        let info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );
    
        let mut surface = Surface::new_raster(&info, None, None).unwrap();
        let canvas = surface.canvas();
    
        for (i, chunk) in pixel_data.chunks_exact(3).enumerate() {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let color4f = Color4f::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
            let paint = Paint::new(color4f, None);
    
            let x = (i as u32) % width;
            let y = (i as u32) / width;
    
            let rect = Rect::from_point_and_size((x as f32, y as f32), (1.0, 1.0));
            canvas.draw_rect(rect, &paint);
        }
    
        let image = surface.image_snapshot();
    
        if let Some(data) = image.encode_to_data(EncodedImageFormat::PNG) {
            let bytes = data.as_bytes();
            
            fs::write(TEMP_RESULT_PATH, bytes).expect("Failed to write image data to file");
        }        
    
        (width, height)
    }     

    fn compress_hif_file(input_path: &PathBuf) -> Result<(), std::io::Error> {
        let input = fs::File::open(input_path)?;
        let mut reader = std::io::BufReader::new(input);
        let output_path = input_path.with_extension("hif.gz");
        let output = fs::File::create(&output_path)?;
        let writer = std::io::BufWriter::new(output);
        let mut encoder = GzEncoder::new(writer, Compression::default());

        let mut buffer = [0; 4096];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            encoder.write_all(&buffer[..count])?;
        }
        encoder.finish()?;
        Ok(())
    }

    fn run_app(file_path: PathBuf) -> Result<(), eframe::Error> {
        let (width, height) = Sam::hif_to_png(file_path);
        let options = eframe::NativeOptions {
            resizable: true,
            initial_window_size: Some(egui::vec2(width as f32, height as f32)),
            ..Default::default()
        };

        eframe::run_native(
            ".hif opener",
            options,
            Box::new(|_cc| Box::<SamPreview>::default()),
        )
    }
}

struct SamPreview {
    image: RetainedImage,
}

impl Default for SamPreview {
    fn default() -> Self {
        let image_data = std::fs::read(TEMP_RESULT_PATH).expect("Failed to read image file");

        fs::remove_file(TEMP_RESULT_PATH).expect("File delete failed on TEMP RESULT PATH");

        Self {
            image: RetainedImage::from_image_bytes(TEMP_RESULT_PATH, &image_data).unwrap(),
        }
    }
}

impl eframe::App for SamPreview {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        println!("thanks for using the .hif file format :3");
        egui::CentralPanel::default().show(ctx, |ui| {
            self.image.show(ui);
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Not enough arguments provided.");
        return Ok(());
    }

    if &args[1] == "compile" {
        if args.len() < 3 {
            eprintln!("Secondary argument ('path') not provided. Example: `cargo run compile <path/img.png>`");
            return Ok(());
        }

        let path: PathBuf = (&args[2]).into();

        match Sam::convert_png_to_hif(path) {
            Ok(()) => println!("Successfully converted PNG to HIF"),
            Err(_) => println!("Failed to convert PNG to HIF"),
        }
    } else if &args[1] == "compress" {
        if args.len() < 3 {
            eprintln!("Path argument not provided. Usage: `cargo run compress path/to/file.hif`");
            return Ok(());
        }
        let path: PathBuf = (&args[2]).into();
        match Sam::compress_hif_file(&path) {
            Ok(()) => println!("Successfully compressed .hif file"),
            Err(e) => println!("Failed to compress .hif file: {}", e),
        }
    } else {
        let file_path: PathBuf = (&args[1]).into();
        Sam::run_app(file_path)?;
    }

    Ok(())
}