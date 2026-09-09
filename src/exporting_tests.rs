use super::*;
fn sprite() -> Document {
    let mut d = Document::new(4, 4);
    d.put(0, 0, 1, 1, [255, 0, 0, 128]);
    d.frames.push(d.frames[0].clone());
    d.frames[1].duration = 200;
    d.frames[1].cels[0][5] = [0, 255, 0, 255];
    d.frames.push(d.frames[0].clone());
    d.frames[2].duration = 300;
    d
}
#[test]
fn every_format_encodes_decodes_and_keeps_source() {
    let d = sprite();
    let original = d.clone();
    let dir = tempfile::tempdir().unwrap();
    for format in [
        Format::Png,
        Format::Jpeg,
        Format::WebP,
        Format::Bmp,
        Format::Tga,
        Format::Gif,
    ] {
        let o = Options {
            format,
            scale: 2,
            ..Default::default()
        };
        let path = dir.path().join(format!("test.{}", format.ext()));
        write(&d, 0, 0, &o, &path).unwrap();
        let img = image::open(path).unwrap().into_rgba8();
        assert_eq!(img.dimensions(), (8, 8));
        if matches!(format, Format::Png | Format::WebP | Format::Tga) {
            assert_eq!(img.get_pixel(2, 2).0, [255, 0, 0, 128]);
        }
        if matches!(format, Format::Jpeg | Format::Bmp) {
            assert_eq!(img.get_pixel(0, 0).0[3], 255);
        }
    }
    assert_eq!(d, original);
}
#[test]
fn trim_range_direction_and_sheet_padding() {
    let d = sprite();
    let o = Options {
        mode: Mode::Sheet,
        scope: Scope::All,
        direction: Direction::PingPong,
        trim: true,
        scale: 3,
        columns: 2,
        padding: 1,
        ..Default::default()
    };
    let p = plan(&d, 0, 0, &o).unwrap();
    assert_eq!(p.frames, vec![0, 1, 2, 1]);
    assert_eq!((p.width, p.height), (3, 3));
    assert_eq!((p.total_width, p.total_height), (9, 9));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sheet.png");
    write(&d, 0, 0, &o, &path).unwrap();
    let image = image::open(path).unwrap().into_rgba8();
    assert_eq!(image.get_pixel(0, 0).0, [0; 4]);
    assert_eq!(image.get_pixel(1, 1).0, [255, 0, 0, 128]);
    assert_eq!(image.get_pixel(5, 1).0, [0, 255, 0, 255]);
}
#[test]
fn sequence_and_gif_keep_frame_order_and_timing() {
    use image::AnimationDecoder;
    let d = sprite();
    let dir = tempfile::tempdir().unwrap();
    let mut o = Options {
        mode: Mode::Sequence,
        scope: Scope::Range,
        first: 1,
        last: 2,
        direction: Direction::Reverse,
        ..Default::default()
    };
    let output = write(&d, 0, 0, &o, dir.path()).unwrap();
    assert!(output.join("frame-0001.png").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("frames.json")).unwrap()).unwrap();
    assert_eq!(manifest["frames"][0]["source_frame"], 2);
    assert_eq!(manifest["frames"][0]["duration_ms"], 200);
    o.mode = Mode::Animation;
    o.format = Format::Gif;
    let gif = dir.path().join("animation.gif");
    write(&d, 0, 0, &o, &gif).unwrap();
    let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
        std::fs::File::open(gif).unwrap(),
    ))
    .unwrap();
    let frames = decoder.into_frames().collect_frames().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].delay().numer_denom_ms(), (200, 1));
}
#[test]
fn invalid_export_does_not_touch_destination() {
    let d = sprite();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected.png");
    std::fs::write(&path, b"existing").unwrap();
    let o = Options {
        scale: 0,
        ..Default::default()
    };
    assert!(write(&d, 0, 0, &o, &path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"existing");
    let o = Options {
        scope: Scope::Range,
        mode: Mode::Sheet,
        first: 2,
        last: 1,
        ..Default::default()
    };
    assert!(plan(&d, 0, 0, &o).is_err());
}
