//! Generated JPEG/EXIF bytes for tests. No camera RAW samples are checked in.

use image::{DynamicImage, RgbImage};

pub fn solid_jpeg(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);
    for _ in 0..width * height {
        pixels.extend_from_slice(&rgb);
    }
    let image = DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, pixels).unwrap());
    let mut bytes = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90);
    image.write_with_encoder(encoder).unwrap();
    bytes
}

/// A small JPEG whose EXIF describes a Canon frame. `orientation` is the tag value 1–8.
pub fn jpeg_with_shooting_exif(width: u32, height: u32, orientation: u16) -> Vec<u8> {
    let jpeg = solid_jpeg(width, height, [180, 40, 40]);
    let tiff = shooting_tiff(width, height, orientation);
    let mut payload = b"Exif\0\0".to_vec();
    payload.extend(tiff);
    let mut app1 = vec![0xFF, 0xE1];
    app1.extend(((payload.len() + 2) as u16).to_be_bytes());
    app1.extend(payload);

    let mut out = Vec::with_capacity(jpeg.len() + app1.len());
    out.extend_from_slice(&jpeg[..2]);
    out.extend(app1);
    out.extend_from_slice(&jpeg[2..]);
    out
}

struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    extra: Option<Vec<u8>>,
    inline: [u8; 4],
}

fn short_entry(tag: u16, value: u16) -> Entry {
    let mut inline = [0; 4];
    inline[..2].copy_from_slice(&value.to_le_bytes());
    Entry {
        tag,
        typ: 3,
        count: 1,
        extra: None,
        inline,
    }
}

fn long_entry(tag: u16, value: u32) -> Entry {
    Entry {
        tag,
        typ: 4,
        count: 1,
        extra: None,
        inline: value.to_le_bytes(),
    }
}

fn ascii_entry(tag: u16, text: &str) -> Entry {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    let count = bytes.len() as u32;
    if bytes.len() <= 4 {
        let mut inline = [0; 4];
        inline[..bytes.len()].copy_from_slice(&bytes);
        Entry {
            tag,
            typ: 2,
            count,
            extra: None,
            inline,
        }
    } else {
        Entry {
            tag,
            typ: 2,
            count,
            extra: Some(bytes),
            inline: [0; 4],
        }
    }
}

fn rational_entry(tag: u16, num: u32, denom: u32) -> Entry {
    let mut extra = Vec::with_capacity(8);
    extra.extend(num.to_le_bytes());
    extra.extend(denom.to_le_bytes());
    Entry {
        tag,
        typ: 5,
        count: 1,
        extra: Some(extra),
        inline: [0; 4],
    }
}

fn extra_size(entries: &[Entry]) -> usize {
    entries.iter().fold(0, |acc, entry| match &entry.extra {
        Some(payload) => {
            let next = acc + payload.len();
            if next % 2 == 1 {
                next + 1
            } else {
                next
            }
        }
        None => acc,
    })
}

fn write_ifd(entries: &[Entry], data_base: u32) -> (Vec<u8>, Vec<u8>) {
    let mut dir = Vec::new();
    dir.extend((entries.len() as u16).to_le_bytes());
    let mut extra = Vec::new();
    for entry in entries {
        dir.extend(entry.tag.to_le_bytes());
        dir.extend(entry.typ.to_le_bytes());
        dir.extend(entry.count.to_le_bytes());
        if let Some(payload) = &entry.extra {
            let offset = data_base + extra.len() as u32;
            dir.extend(offset.to_le_bytes());
            extra.extend(payload);
            if extra.len() % 2 == 1 {
                extra.push(0);
            }
        } else {
            dir.extend(entry.inline);
        }
    }
    dir.extend(0u32.to_le_bytes());
    (dir, extra)
}

fn shooting_tiff(width: u32, height: u32, orientation: u16) -> Vec<u8> {
    let mut ifd0 = vec![
        long_entry(0x0100, width),
        long_entry(0x0101, height),
        ascii_entry(0x010F, "Canon"),
        ascii_entry(0x0110, "EOS R5"),
        short_entry(0x0112, orientation),
        long_entry(0x8769, 0),
    ];
    let mut exif = vec![
        rational_entry(0x829A, 1, 200),
        rational_entry(0x829D, 12, 10),
        short_entry(0x8827, 400),
        ascii_entry(0x9003, "2024:01:15 14:30:45"),
        rational_entry(0x920A, 85, 1),
        ascii_entry(0x9291, "12"),
        long_entry(0xA002, width),
        long_entry(0xA003, height),
        ascii_entry(0xA434, "RF85mm F1.2 L USM"),
    ];
    ifd0.sort_by_key(|entry| entry.tag);
    exif.sort_by_key(|entry| entry.tag);

    let ifd0_dir_len = 2 + ifd0.len() * 12 + 4;
    let exif_off = (8 + ifd0_dir_len + extra_size(&ifd0)) as u32;
    for entry in &mut ifd0 {
        if entry.tag == 0x8769 {
            entry.inline = exif_off.to_le_bytes();
        }
    }

    let exif_dir_len = 2 + exif.len() * 12 + 4;
    let (ifd0_dir, ifd0_extra) = write_ifd(&ifd0, 8 + ifd0_dir_len as u32);
    let (exif_dir, exif_extra) = write_ifd(&exif, exif_off + exif_dir_len as u32);

    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II");
    tiff.extend(42u16.to_le_bytes());
    tiff.extend(8u32.to_le_bytes());
    tiff.extend(ifd0_dir);
    tiff.extend(ifd0_extra);
    tiff.extend(exif_dir);
    tiff.extend(exif_extra);
    tiff
}
