use std::borrow::Cow;
use std::convert::TryInto;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::AppState;
use super::command::Error;
use super::instruction::{Instruction, Category};

/// Encode a buffer of pixel data as a PNG file and write it to `w`.
fn write_rgba_image_data(
    w: impl Write,
    width: usize,
    height: usize,
    data: &[u8]
) -> Result<(), png::EncodingError> {
    use png::HasParameters as _;
    debug_assert_eq!(width * 4 * height, data.len());
    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(data)
}

fn write_gif_data<'a>(
    path: &Path,
    width: u16,
    height: u16,
    num_frames: usize,
    palette: &[u8],
    mut update_frame: impl FnMut(usize, &mut Vec<u8>),
) -> std::io::Result<()> {
    use gif::SetParameter as _;
    let mut encoder = gif::Encoder::new(File::create(path)?, width, height, palette)?;
    encoder.set(gif::Repeat::Infinite)?;
    let mut frame_data = Vec::with_capacity(width as usize * height as usize * 4);
    for i in 0..num_frames {
        update_frame(i, &mut frame_data);
        encoder.write_frame(&gif::Frame {
            buffer: Cow::Borrowed(&frame_data),
            width,
            height,
            ..gif::Frame::default()
        })?;
    }
    Ok(())
}

impl<W: Write> AppState<W> {
    pub fn write_image_data(&mut self, path: PathBuf) -> Result<(), Error> {
        if path.exists() {
            return Err(Error::ExportFileExists(path));
        }

        let file = File::create(&path).map_err(|_| Error::ExportFailure(path.clone()))?;

        let width  = self.grid.width();
        let height = self.grid.height();

        let mut data = Vec::with_capacity(width as usize * height as usize * 4);
        for ins in self.grid.view_all() {
            let [r, g, b] = Instruction::from_byte(ins).category().color_rgb();
            data.extend_from_slice(&[r, g, b, 0xff]);
        }

        write_rgba_image_data(file, width, height, &data)
            .map_err(|_| Error::ExportFailure(path))
    }
    pub fn write_gif_data(
        &mut self,
        path: PathBuf,
        num_frames: usize,
        step: usize
    ) -> Result<(), Error> {
        // Make sure we're in a reasonable state
        if path.exists() {
            return Err(Error::ExportFileExists(path));
        }
        let width: u16 = self.grid.width().try_into().map_err(|_| Error::WorldTooBig)?;
        let height: u16 = self.grid.height().try_into().map_err(|_| Error::WorldTooBig)?;

        // Compute and write the frames
        write_gif_data(&path, width, height, num_frames, &Category::PALETTE, |i, frame_data| {
            if i != 0 {
                frame_data.clear();
                for _ in 0..step {
                    self.cycle();
                }
            }
            for ins in self.grid.view_all() {
                frame_data.push(Instruction::from_byte(ins).category() as u8);
            }
        }).map_err(|_| Error::ExportFailure(path))        
    }
}