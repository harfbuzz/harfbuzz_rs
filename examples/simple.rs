extern crate harfbuzz_rs;

use harfbuzz_rs::{Face, UnicodeBuffer};

fn main() {
    let index = 0;
    let path = "testfiles/Optima.ttc";
    let mut font = Face::from_file(path, index)
        .expect("Error reading font file.")
        .create_font();

    font.set_scale(20 * 64, 20 * 64);
    font.set_ppem(72 * 64, 72 * 64);

    // Create a buffer with some text and shape it...
    let result = UnicodeBuffer::new().add_str("Hello World!").shape(&font, &[]);

    // ... and get the results.
    let positions = result.get_glyph_positions();
    let infos = result.get_glyph_infos();

    // iterate over the shaped glyphs
    for (position, info) in positions.iter().zip(infos) {
        let gid = info.codepoint;
        let cluster = info.cluster;
        let x_advance = position.x_advance;
        let x_offset = position.x_offset;
        let y_offset = position.y_offset;

        println!("gid{:0>2?}={:0>2?}@{:>4?},{:?}+{:?}",
                 gid,
                 cluster,
                 x_advance,
                 x_offset,
                 y_offset);
    }
}
