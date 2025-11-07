#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! aseprite-reader = { path = "../../../../Github/aseprite-reader/" }
//! ```
#![allow(unused_doc_comments)]

use aseprite_reader::*;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};

const GLYPHS_FP: &str = "sitelen pona.aseprite";
const VOCAB_FP: &str = "vocab.txt";
const GLYPHS_BIN_FP: &str = "gen/glyphs.bin";
const JACKAL_CODE_FP: &str = "gen/toki_pona_data.jkl";
const EXAMPLE_FP: &str = "gen/example.txt";

fn main() {
    let mut p = std::path::Path::new(&std::env::args().nth(0).unwrap())
        .canonicalize()
        .unwrap();
    p.pop();
    std::env::set_current_dir(p).unwrap();
    create_glyphs_bin();
    create_data_table();
}

fn create_glyphs_bin() {
    let file_data = std::fs::read(GLYPHS_FP).unwrap();
    let image = parse(&file_data).unwrap();
    let glyphs_bin = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(GLYPHS_BIN_FP)
        .unwrap();
    let mut glyphs_bin = BufWriter::new(glyphs_bin);
    let width = image.header.width;
    let height = image.header.height;
    assert_eq!(width, 128);
    assert_eq!(height, 160);
    let mut get_pixel = {
        let frame = &image.frames[0];
        let glyph_layer_index = frame
            .chunks
            .iter()
            .filter_map(|chunk| match chunk {
                Chunk::Layer { name, .. } => Some(name),
                _ => None,
            })
            .position(|name| name == "Sitelen Pona")
            .unwrap();
        frame
            .chunks
            .iter()
            .find_map(|chunk| match chunk {
                &Chunk::Cel {
                    layer_index,
                    x_pos,
                    y_pos,
                    width: layer_width,
                    height: layer_height,
                    ref pixels,
                    ..
                } if usize::from(layer_index) == glyph_layer_index => {
                    let x_pos = isize::from(x_pos);
                    let y_pos = isize::from(y_pos);
                    let layer_width = layer_width as isize;
                    let layer_height = layer_height as isize;
                    let mut used_color = None;
                    //println!("{} -- {x_pos} {y_pos} {layer_width} {layer_height} -- {}", pixels.len(), layer_width * layer_height);
                    let get_pixel = move |x: isize, y: isize| {
                        //dbg!((x,y,x_pos,y_pos,layer_width,layer_height));
                        let ix = x - x_pos;
                        let iy = y - y_pos;
                        if ix >= 0 && iy >= 0 && ix < layer_width && iy < layer_height {
                            let pixel = pixels[(iy * layer_width + ix) as usize];
                            if pixel.alpha != 0 {
                                match used_color {
                                    None => used_color = Some(pixel),
                                    Some(previous) => assert_eq!(previous, pixel),
                                }
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };
                    Some(get_pixel)
                }
                _ => None,
            })
            .unwrap()
    };
    let tile_width = 8;
    let tile_height = 16;
    for tile_y in 0..height as isize / tile_height {
        for tile_x in 0..width as isize / tile_width {
            //println!("{tile_x} {tile_y}");
            for py in 0..tile_height {
                let y = py + tile_y * tile_height;
                let byte: u8 = (0..tile_width)
                    .map(|px| {
                        let x = px + tile_x * tile_width;
                        u8::from(get_pixel(x, y)) << px
                    })
                    .sum();
                glyphs_bin.write(&[byte]).unwrap();
            }
        }
    }
    glyphs_bin.flush().unwrap();
}

fn create_data_table() {
    let vocab = std::fs::read_to_string(VOCAB_FP).unwrap();
    let jackal_code = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(JACKAL_CODE_FP)
        .unwrap();
    let mut jackal_code = BufWriter::new(jackal_code);
    let example_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(EXAMPLE_FP)
        .unwrap();
    let mut example_file = BufWriter::new(example_file);
    let mut used_encodings = BTreeSet::<u8>::new();
    let mut punctuation = BTreeMap::new();
    let mut alternates = BTreeMap::new();
    let mut words = BTreeMap::new();
    let mut mappings = BTreeMap::<isize, Vec<u8>>::new();
    for ((i, word), mut encoding) in vocab
        .lines()
        .enumerate()
        .zip(128_usize..)
        .filter(|((_, word), _)| !word.is_empty())
    {
        if encoding > 255 {
            encoding -= 255;
            if encoding >= 0x0A {
                encoding += 1;
            }
        }
        let encoding = u8::try_from(encoding).unwrap();
        assert!(
            used_encodings.insert(encoding),
            "duplicate encoding for {word} {encoding} {used_encodings:?}"
        );
        assert!(
            !(0x20..=0x7E).contains(&encoding) && encoding != 0 && encoding != 0x0a,
            "encoding for {word} conflicts with ascii: 0x{encoding:02x}"
        );
        if let Some(punct) = word.strip_prefix(":") {
            let key = punct.chars().next().unwrap();
            let mut punct_name = punct[1..].to_owned();
            punct_name.make_ascii_uppercase();
            punctuation.insert(encoding, (key, punct_name));
        } else if let Some(alternate) = word.strip_prefix("+") {
            alternates.insert(words[alternate], encoding);
        } else {
            words.insert(word.to_owned(), encoding);
        }
        mappings
            .entry(i as isize - encoding as isize)
            .or_default()
            .push(encoding);
    }
    for encodings in mappings.values_mut() {
        encodings.sort();
    }
    let mapping_ranges: BTreeMap<Vec<(u8, u8)>, isize> = mappings
        .into_iter()
        .map(|(diff, encodings)| {
            let mut ranges = vec![];
            let mut start = encodings[0];
            let mut end = start;
            for &e in &encodings[1..] {
                if end as isize != e as isize - 1 {
                    ranges.push((start, end));
                    start = e;
                }
                end = e;
            }
            ranges.push((start, end));
            assert!(ranges.is_sorted());
            (ranges, diff)
        })
        .collect();
    // add alternative spelling of "ale"
    {
        let ale_index = words["ale"];
        words.insert("ali".to_owned(), ale_index);
    }
    //eprintln!("{words:?}");
    //eprintln!("{punctuation:?}");
    //eprintln!("{alternates:?}");

    // what girls will do when no try block
    let meow: Result<(), std::io::Error> = (|| {
        write!(
            jackal_code,
            "//! THIS FILE IS AUTOGENERATED BY `gen.rs`\n\n\n"
        )?;
        write!(jackal_code, "toki_dict_words : ^UBYTE[] = {{")?;
        for word in words.keys() {
            write!(jackal_code, "{word:?},")?;
        }
        write!(jackal_code, "NULLPTR}}\n\n")?;

        write!(jackal_code, "toki_dict_encodings : UBYTE[] = {{")?;
        for encoding in words.values() {
            write!(jackal_code, "{encoding},")?;
        }
        write!(jackal_code, "}}\n\n")?;

        writeln!(
            jackal_code,
            "FN TokiAlternate (\n    IN c : UBYTE,\n) : UBYTE"
        )?;
        for (from, to) in &alternates {
            writeln!(jackal_code, "    IF c == {from} THEN RETURN {to} END")?;
        }
        write!(jackal_code, "    RETURN c\nEND\n\n")?;

        writeln!(
            jackal_code,
            "FN TokiGetPunct (\n    IN c : UBYTE,\n) : UBYTE"
        )?;
        for (i, (encoding, (key, punct_name))) in punctuation.iter().enumerate() {
            let if_kw = if i == 0 { "IF" } else { "ELSEIF" };
            writeln!(
                jackal_code,
                "    {if_kw} c == {key:?} THEN RETURN {encoding} // {punct_name}"
            )?;
        }
        writeln!(jackal_code, "    ELSE RETURN c END\nEND\n\n")?;

        writeln!(
            jackal_code,
            "FN TokiEncodingToTileIndex (\n    IN encoding : UBYTE,\n) : UBYTE"
        )?;
        for (ranges, &offset) in &mapping_ranges {
            write!(jackal_code, "    IF ")?;
            let mut write_or = false;
            for &(from, to) in ranges {
                if write_or {
                    write!(jackal_code, "OR ")?;
                } else {
                    write_or = true;
                }
                write!(jackal_code, "{from} <= encoding AND encoding <= {to} ")?;
            }
            write!(jackal_code, "THEN\n        RETURN (encoding ")?;
            if offset < 0 {
                write!(jackal_code, "- {}", -offset)?;
            } else {
                write!(jackal_code, "+ {offset}")?;
            }
            writeln!(jackal_code, ") & 255\n    END")?;
        }
        writeln!(jackal_code, "    RETURN 255\nEND\n\n")?;
        {
            writeln!(
                jackal_code,
                "FN TokiIsValidChar (\n    IN c : UBYTE,\n) : UBYTE"
            )?;
            let used_chars: BTreeSet<_> = words.keys().flat_map(|word| word.chars()).collect();
            write!(jackal_code, "    RETURN ")?;
            for (i, c) in used_chars.into_iter().enumerate() {
                if i != 0 {
                    write!(jackal_code, " OR ")?;
                }
                write!(jackal_code, "c == {c:?}")?;
            }
            write!(jackal_code, "\nEND\n\n")?;
        }
        ////// example file

        //for (word, &encoding) in &words {
        //    example_file.write_all(&[encoding])?;
        //    write!(example_file, " {word}, ")?;
        //}
        for i in 0..=u8::MAX {
            example_file.write_all(&[i])?;
            if i % 16 == 15 {
                writeln!(example_file)?;
            }
        }
        Ok(())
    })();
    meow.unwrap();
}
