use png::Encoder;
use png::HasParameters as _;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use super::AppState;
use super::command::Error;
use super::instruction::Instruction;

/// Encode a buffer of pixel data as a PNG file and write it to `w`.
pub fn write_rgba_image_data(
    w: impl Write,
    path: PathBuf,
    width: usize,
    height: usize,
    data: &[u8]
) -> Result<(), Error> {
    debug_assert_eq!(width * 4 * height, data.len());
    let mut encoder = Encoder::new(w, width as u32, height as u32);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    encoder.write_header()
        .map_err(|_| Error::ExportFailure(path.clone()))?
        .write_image_data(data)
        .map_err(|_| Error::ExportFailure(path))
}

impl<W: Write> AppState<W> {
    pub fn write_image_data(&mut self, path: PathBuf, pixel_scale: u8) -> Result<(), Error> {
        if path.exists() {
            return Err(Error::ExportFileExists(path));
        }

        let file = File::create(&path).map_err(|_| Error::ExportFailure(path.clone()))?;

        let pixel_scale = pixel_scale as usize;
        let image_width  = self.grid.width()  * pixel_scale;
        let image_height = self.grid.height() * pixel_scale;
        
        let mut data        = Vec::with_capacity(image_width * image_height * 4);
        let mut current_row = Vec::with_capacity(image_width * 4);

        for row in self.grid.view_all() {
            current_row.clear();
            for (_, ins) in row {
                let [r, g, b] = Instruction::from_byte(ins).category().color_rgb();
                current_row.extend_from_slice(&[r, g, b, 0xff].repeat(pixel_scale));
            }
            for _ in 0..pixel_scale {
                data.extend_from_slice(&current_row);
            }
        }

        write_rgba_image_data(file, path, image_width, image_height, &data)
    }
}