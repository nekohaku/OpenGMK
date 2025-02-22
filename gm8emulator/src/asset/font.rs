use crate::{
    asset::Sprite,
    gml,
    render::{
        atlas::{AtlasBuilder, AtlasRef},
        Renderer,
    },
};
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Font {
    pub name: gml::String,
    pub sys_name: gml::String,
    pub charset: u32,
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
    pub first: u8,
    pub last: u8,
    pub tallest_char_height: u32,
    pub chars: Box<[Character]>,
    pub own_graphics: bool, // Does this Font own the graphics associated with it?
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Character {
    pub offset: i32,
    pub distance: i32,
    pub atlas_ref: AtlasRef,
}

impl Font {
    pub fn get_char(&self, index: u8) -> Option<Character> {
        if let Some(index) = index.checked_sub(self.first) { self.chars.get(index as usize).copied() } else { None }
    }

    pub fn get_encoding(&self, default: &'static Encoding) -> &'static Encoding {
        match self.charset {
            0x00 => encoding_rs::WINDOWS_1252, // ANSI_CHARSET
            0x80 => encoding_rs::SHIFT_JIS,    // SHIFTJIS_CHARSET
            0x81 => encoding_rs::EUC_KR,       // HANGUL_CHARSET
            0x82 => default,                   // JOHAB_CHARSET
            0x86 => encoding_rs::GBK,          // GB2312_CHARSET
            0x88 => encoding_rs::BIG5,         // CHINESEBIG5_CHARSET
            0xA1 => encoding_rs::WINDOWS_1253, // GREEK_CHARSET
            0xA2 => encoding_rs::WINDOWS_1254, // TURKISH_CHARSET
            0xA3 => encoding_rs::WINDOWS_1258, // VIETNAMESE_CHARSET
            0xB1 => encoding_rs::WINDOWS_1255, // HEBREW_CHARSET
            0xB2 => encoding_rs::WINDOWS_1256, // ARABIC_CHARSET
            0xBA => encoding_rs::WINDOWS_1257, // BALTIC_CHARSET
            0xCC => encoding_rs::WINDOWS_1251, // RUSSIAN_CHARSET
            0xDE => encoding_rs::WINDOWS_874,  // THAI_CHARSET
            0xEE => encoding_rs::WINDOWS_1250, // EASTEUROPE_CHARSET
            _ => default,
        }
    }
}

pub fn load_default_font(atlases: &mut AtlasBuilder) -> Result<Font, String> {
    // In GM8, the default font is Arial at size 12, but Arial is nonfree,
    // so we instead went for a free alternative called Arimo, under Apache 2.0. https://fonts.google.com/specimen/Arimo
    // arimo.dat was generated by importing Arimo into GM8 and exporting the resulting font data.
    // The `offset` field was tweaked to be closer to Arial's.
    let data = include_bytes!("../../data/arimo.dat");
    let mut chars = Vec::with_capacity(0x60);
    let mut tallest_char_height = 0;
    let mut cursor = 0;
    for _ in 0..0x60 {
        let offset = data[cursor] as i8 as i32;
        let distance = data[cursor + 1] as i8 as i32;
        let width = data[cursor + 2] as u32;
        let height = data[cursor + 3] as u32;
        cursor += 4;
        if height > tallest_char_height {
            tallest_char_height = height;
        }
        let size = (width * height) as usize;
        let mut char = Vec::with_capacity(size * 4);
        for i in 0..size {
            char.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
            char.push(data[cursor + i]);
        }
        cursor += size;
        let atlas_ref = atlases
            .texture(width as _, height as _, 0, 0, char.into_boxed_slice())
            .ok_or("Couldn't pack default font")?;
        chars.push(Character { offset, distance, atlas_ref });
    }
    Ok(Font {
        name: b"default_font".as_ref().into(),
        sys_name: b"Arimo".as_ref().into(),
        charset: 1,
        size: 12,
        bold: false,
        italic: false,
        first: 0x20,
        last: 0x7f,
        tallest_char_height,
        chars: chars.into_boxed_slice(),
        own_graphics: true,
    })
}

pub fn create_chars_from_sprite(sprite: &Sprite, prop: bool, sep: i32, renderer: &Renderer) -> Box<[Character]> {
    let mut chars = Vec::with_capacity(sprite.frames.len());
    if prop {
        // proportional font, get the left and right bounds of each character
        for frame in &sprite.frames {
            let data = renderer.dump_sprite(&frame.atlas_ref);
            let column_empty =
                |&x: &u32| (0..sprite.height).any(|y| data[(y * sprite.width + x) as usize * 4 + 3] != 0);
            let left_edge = (0..sprite.width).find(column_empty).map(|x| x as i32).unwrap_or(sprite.width as i32 - 1);
            let right_edge = (0..sprite.width).rfind(column_empty).unwrap_or(0) as i32;
            chars.push(Character {
                offset: right_edge + sep - left_edge,
                distance: -left_edge,
                atlas_ref: frame.atlas_ref.clone(),
            });
        }
    } else {
        // non-proportional font, just add them whole
        chars.extend(sprite.frames.iter().map(|f| Character {
            offset: f.width as i32 + sep,
            distance: 0,
            atlas_ref: f.atlas_ref.clone(),
        }));
    }
    chars.into_boxed_slice()
}
