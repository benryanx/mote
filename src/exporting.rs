use crate::Studio;
use eframe::egui;
use image::{ImageFormat, RgbaImage};
use mote::document::{CLEAR, Document, over};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
    Bmp,
    Tga,
    Gif,
}
impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
            Self::Tga => "tga",
            Self::Gif => "gif",
        }
    }
    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::WebP => ImageFormat::WebP,
            Self::Bmp => ImageFormat::Bmp,
            Self::Tga => ImageFormat::Tga,
            Self::Gif => ImageFormat::Gif,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    Image,
    Sheet,
    Sequence,
    Animation,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Scope {
    Current,
    All,
    Range,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Reverse,
    PingPong,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Options {
    pub format: Format,
    pub mode: Mode,
    pub scope: Scope,
    pub first: usize,
    pub last: usize,
    pub scale: u32,
    pub direction: Direction,
    pub active_layer: bool,
    pub trim: bool,
    pub columns: usize,
    pub padding: u32,
    pub quality: u8,
    pub matte: [u8; 3],
    pub opaque: bool,
    pub looped: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            format: Format::Png,
            mode: Mode::Image,
            scope: Scope::Current,
            first: 1,
            last: 1,
            scale: 1,
            direction: Direction::Forward,
            active_layer: false,
            trim: false,
            columns: 8,
            padding: 0,
            quality: 95,
            matte: [255; 3],
            opaque: false,
            looped: true,
        }
    }
}
pub struct Plan {
    pub frames: Vec<usize>,
    crop: (u32, u32, u32, u32),
    pub width: u32,
    pub height: u32,
    pub total_width: u32,
    pub total_height: u32,
    columns: usize,
}
fn pixels(doc: &Document, frame: usize, layer: Option<usize>) -> Vec<[u8; 4]> {
    if let Some(layer) = layer {
        doc.frames[frame].cels[layer]
            .iter()
            .map(|p| over(CLEAR, *p, doc.layers[layer].opacity))
            .collect()
    } else {
        doc.composite(frame)
    }
}
pub fn plan(doc: &Document, active: usize, layer: usize, o: &Options) -> Result<Plan, String> {
    doc.validate()?;
    if active >= doc.frames.len()
        || layer >= doc.layers.len()
        || !(1..=16).contains(&o.scale)
        || !(1..=100).contains(&o.quality)
        || o.columns == 0
        || o.columns > 1024
        || o.padding > 64
    {
        return Err("Invalid export settings".into());
    }
    if o.mode == Mode::Animation && o.format != Format::Gif {
        return Err(
            "Animation output requires GIF. Use an image sequence for other formats.".into(),
        );
    }
    let mut frames = if o.mode == Mode::Image || o.scope == Scope::Current {
        vec![active]
    } else if o.scope == Scope::All {
        (0..doc.frames.len()).collect()
    } else {
        if o.first == 0 || o.first > o.last || o.last > doc.frames.len() {
            return Err("Frame range must be within the active document.".into());
        }
        (o.first - 1..o.last).collect::<Vec<_>>()
    };
    match o.direction {
        Direction::Reverse => frames.reverse(),
        Direction::PingPong if frames.len() > 2 => {
            let tail = frames[1..frames.len() - 1]
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>();
            frames.extend(tail);
        }
        _ => {}
    }
    let mut crop = (0, 0, doc.width as u32, doc.height as u32);
    if o.trim {
        let (mut left, mut top, mut right, mut bottom) = (doc.width, doc.height, 0, 0);
        let mut found = false;
        for &f in &frames {
            for (i, p) in pixels(doc, f, o.active_layer.then_some(layer))
                .iter()
                .enumerate()
            {
                if p[3] > 0 {
                    let x = i % doc.width;
                    let y = i / doc.width;
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                    found = true;
                }
            }
        }
        crop = if found {
            (
                left as u32,
                top as u32,
                (right - left + 1) as u32,
                (bottom - top + 1) as u32,
            )
        } else {
            (0, 0, 1, 1)
        };
    }
    let width = crop.2 * o.scale;
    let height = crop.3 * o.scale;
    let columns = o.columns.min(frames.len());
    let (tw, th) = if o.mode == Mode::Sheet {
        let rows = frames.len().div_ceil(columns);
        (
            (width + o.padding) * columns as u32 + o.padding,
            (height + o.padding) * rows as u32 + o.padding,
        )
    } else {
        (width, height)
    };
    let total = if o.mode == Mode::Sheet {
        tw as u64 * th as u64
    } else {
        width as u64 * height as u64 * frames.len() as u64
    };
    if tw > 16384 || th > 16384 || total > 64_000_000 {
        return Err("Export exceeds 16,384 pixels per side or 64 million total output pixels. Reduce scale or frame range.".into());
    }
    Ok(Plan {
        frames,
        crop,
        width,
        height,
        total_width: tw,
        total_height: th,
        columns,
    })
}
fn frame_image(doc: &Document, f: usize, layer: usize, o: &Options, p: &Plan) -> RgbaImage {
    let source = pixels(doc, f, o.active_layer.then_some(layer));
    let mut image = RgbaImage::new(p.width, p.height);
    for y in 0..p.height {
        for x in 0..p.width {
            let sx = p.crop.0 + x / o.scale;
            let sy = p.crop.1 + y / o.scale;
            image.put_pixel(
                x,
                y,
                image::Rgba(source[sy as usize * doc.width + sx as usize]),
            );
        }
    }
    image
}
fn flatten(image: &mut RgbaImage, matte: [u8; 3]) {
    for pixel in image.pixels_mut() {
        pixel.0 = over([matte[0], matte[1], matte[2], 255], pixel.0, 255);
    }
}
fn write_image(mut img: RgbaImage, o: &Options, file: &mut std::fs::File) -> Result<(), String> {
    if o.opaque || matches!(o.format, Format::Jpeg | Format::Bmp) {
        flatten(&mut img, o.matte);
    }
    if o.format == Format::Jpeg {
        let rgb = image::DynamicImage::ImageRgba8(img).into_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(file, o.quality)
            .encode_image(&rgb)
            .map_err(|e| e.to_string())
    } else {
        img.write_to(file, o.format.image_format())
            .map_err(|e| e.to_string())
    }
}
pub fn write(
    doc: &Document,
    active: usize,
    layer: usize,
    o: &Options,
    path: &Path,
) -> Result<PathBuf, String> {
    let p = plan(doc, active, layer, o)?;
    if o.mode == Mode::Sequence {
        // Stage the complete sequence in its own new directory. Existing files are never replaced.
        let stage = tempfile::Builder::new()
            .prefix("mote-frames-")
            .tempdir_in(path)
            .map_err(|e| e.to_string())?;
        let mut manifest = vec![];
        for (i, &f) in p.frames.iter().enumerate() {
            let name = format!("frame-{:04}.{}", i + 1, o.format.ext());
            let mut file =
                std::fs::File::create(stage.path().join(&name)).map_err(|e| e.to_string())?;
            write_image(frame_image(doc, f, layer, o, &p), o, &mut file)?;
            file.sync_all().map_err(|e| e.to_string())?;
            manifest.push(serde_json::json!({"file":name,"source_frame":f+1,"duration_ms":doc.frames[f].duration}));
        }
        let file =
            std::fs::File::create(stage.path().join("frames.json")).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(
            &file,
            &serde_json::json!({"width":p.width,"height":p.height,"frames":manifest}),
        )
        .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        return Ok(stage.keep());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    if o.mode == Mode::Animation {
        let mut encoder = image::codecs::gif::GifEncoder::new(temp.as_file_mut());
        if o.looped {
            encoder
                .set_repeat(image::codecs::gif::Repeat::Infinite)
                .map_err(|e| e.to_string())?;
        }
        for &f in &p.frames {
            let mut img = frame_image(doc, f, layer, o, &p);
            if o.opaque {
                flatten(&mut img, o.matte);
            }
            encoder
                .encode_frame(image::Frame::from_parts(
                    img,
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(doc.frames[f].duration, 1),
                ))
                .map_err(|e| e.to_string())?;
        }
    } else {
        let img = if o.mode == Mode::Sheet {
            let mut sheet = RgbaImage::new(p.total_width, p.total_height);
            for (i, &f) in p.frames.iter().enumerate() {
                let img = frame_image(doc, f, layer, o, &p);
                let x = o.padding + (i % p.columns) as u32 * (p.width + o.padding);
                let y = o.padding + (i / p.columns) as u32 * (p.height + o.padding);
                image::imageops::replace(&mut sheet, &img, x as i64, y as i64);
            }
            sheet
        } else {
            frame_image(doc, p.frames[0], layer, o, &p)
        };
        write_image(img, o, temp.as_file_mut())?;
    }
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(path.into())
}
impl Studio {
    pub fn open_export(&mut self) {
        self.finish_gesture();
        self.playing = false;
        self.export_options.last = self.export_options.last.clamp(1, self.doc.frames.len());
        self.export_options.first = self.export_options.first.clamp(1, self.export_options.last);
        self.export_open = true;
    }
    pub fn export_dialog(&mut self, ctx: &egui::Context) {
        if !self.export_open {
            return;
        }
        let mut o = self.export_options.clone();
        let mut close = false;
        egui::Modal::new(egui::Id::new("export_options")).show(ctx,|ui|{
            ui.set_width(490.0_f32.min(ctx.content_rect().width()-48.0));
            ui.heading("Export As");ui.label(self.document_title(self.active_document));
            egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-180.0).max(140.0)).show(ui,|ui|{
                ui.horizontal(|ui|{ui.label("Format");egui::ComboBox::from_id_salt("export_format").selected_text(format!("{:?}",o.format)).show_ui(ui,|ui|{for f in [Format::Png,Format::Jpeg,Format::WebP,Format::Bmp,Format::Tga,Format::Gif]{if ui.selectable_value(&mut o.format,f,format!("{f:?}")).changed()&&f!=Format::Gif&&o.mode==Mode::Animation{o.mode=Mode::Image;}}});});
                ui.horizontal_wrapped(|ui|{for(mode,label)in [(Mode::Image,"Single frame"),(Mode::Sheet,"Sprite sheet"),(Mode::Sequence,"Image sequence"),(Mode::Animation,"Animation")]{if ui.add_enabled(mode!=Mode::Animation||o.format==Format::Gif,egui::Button::new(label).selected(o.mode==mode)).clicked(){o.mode=mode;if mode!=Mode::Image{o.scope=Scope::All;}}}});
                if o.mode!=Mode::Image{
                    ui.horizontal(|ui|{ui.label("Frames");for(scope,label)in [(Scope::Current,"Current"),(Scope::All,"All"),(Scope::Range,"Range")]{ui.selectable_value(&mut o.scope,scope,label);}});
                    if o.scope==Scope::Range{ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut o.first).range(1..=self.doc.frames.len()));ui.label("through");ui.add(egui::DragValue::new(&mut o.last).range(1..=self.doc.frames.len()));});}
                    ui.horizontal(|ui|{for(d,label)in [(Direction::Forward,"Forward"),(Direction::Reverse,"Reverse"),(Direction::PingPong,"Ping-pong")]{ui.selectable_value(&mut o.direction,d,label);}});
                }
                ui.add(egui::Slider::new(&mut o.scale,1..=16).text("Scale × (nearest neighbor)"));
                ui.checkbox(&mut o.active_layer,"Export active layer only (including hidden)");ui.checkbox(&mut o.trim,"Trim transparent borders (shared bounds across frames)");
                if o.mode==Mode::Sheet{ui.horizontal(|ui|{ui.label("Columns");ui.add(egui::DragValue::new(&mut o.columns).range(1..=1024));ui.label("Padding px");ui.add(egui::DragValue::new(&mut o.padding).range(0..=64));});}
                if o.format==Format::Jpeg{ui.add(egui::Slider::new(&mut o.quality,1..=100).text("JPEG quality"));}
                if matches!(o.format,Format::Jpeg|Format::Bmp){ui.label("This format uses an opaque background.");}else{ui.checkbox(&mut o.opaque,"Flatten transparency onto background");}
                if o.opaque||matches!(o.format,Format::Jpeg|Format::Bmp){ui.horizontal(|ui|{ui.label("Background");ui.color_edit_button_srgb(&mut o.matte);});}
                if o.format==Format::Gif{ui.small("GIF uses a limited palette, binary transparency and 10 ms timing precision.");if o.mode==Mode::Animation{ui.checkbox(&mut o.looped,"Loop animation");}}
                if o.format==Format::WebP{ui.small("WebP uses lossless encoding. Animation is available through GIF.");}
                if o.mode==Mode::Sequence{ui.small("Choose a parent folder. A new mote-frames-* folder contains numbered images and frame timing JSON.");}
            });
            let result=plan(&self.doc,self.frame,self.layer,&o);
            match &result{Ok(p)=>{ui.label(format!("{} × {} px · {} output frame(s)",p.total_width,p.total_height,p.frames.len()));},Err(e)=>{ui.colored_label(ui.visuals().error_fg_color,e);}}
            ui.horizontal(|ui|{
                if ui.add_enabled(result.is_ok(),egui::Button::new("Export…")).clicked(){
                    let options=o.clone();let doc=self.doc.clone();let active=self.frame;let layer=self.layer;let filename=self.filename(o.format.ext());
                    self.start_background("Exporting artwork",move||{
                    let o=options;
                    let path=if o.mode==Mode::Sequence{rfd::FileDialog::new().pick_folder()}else{rfd::FileDialog::new().add_filter(o.format.ext(),&[o.format.ext()]).set_file_name(filename).save_file()};
                    let Some(path)=path else{return Ok(crate::background::Outcome::Canceled);};
                        let extension=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase();
                        if o.mode!=Mode::Sequence&&extension!=o.format.ext()&&!(o.format==Format::Jpeg&&extension=="jpeg"){return Err(format!("Use the .{} extension for this format.",o.format.ext()));}
                        write(&doc,active,layer,&o,&path).map(crate::background::Outcome::Exported)
                    });
                }
                if ui.button("Cancel").clicked(){close=true;}
            });
            ui.label(&self.status);
        });
        self.export_options = o;
        if close {
            self.export_open = false;
        }
    }
}

#[cfg(test)]
#[path = "exporting_tests.rs"]
mod tests;
