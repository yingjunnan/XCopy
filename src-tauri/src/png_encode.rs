/// Encode RGBA pixel data into a PNG.
///
/// `arboard` already normalizes the platform-native clipboard format
/// (BGRA on Windows) into standard RGBA before handing it to us, so the
/// bytes are used as-is without any channel swapping.
pub fn rgba_to_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    // The image is already RGBA; copy it straight through so channels aren't
    // swapped a second time (which is what produced the red/blue inversion).
    let rgba: Vec<u8> = bytes
        .chunks(4)
        .flat_map(|chunk| match chunk.len() {
            4 => chunk.to_vec(),
            3 => vec![chunk[0], chunk[1], chunk[2], 255],
            _ => chunk.to_vec(),
        })
        .collect();

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr_data.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr_data.push(8);
    ihdr_data.push(6);
    ihdr_data.push(0);
    ihdr_data.push(0);
    ihdr_data.push(0);
    write_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // IDAT
    let mut raw_data = Vec::with_capacity(height + rgba.len());
    for y in 0..height {
        raw_data.push(0);
        let start = y * width * 4;
        let end = start + width * 4;
        raw_data.extend_from_slice(&rgba[start..end.min(rgba.len())]);
    }

    let compressed = deflate(&raw_data);
    write_png_chunk(&mut png, b"IDAT", &compressed);
    write_png_chunk(&mut png, b"IEND", &[]);

    Ok(png)
}

fn write_png_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);
    let crc = crc32(chunk_type, data);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc_table = [0u32; 256];
    for (i, entry) in crc_table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xedb88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        *entry = c;
    }

    let mut crc: u32 = 0xffffffff;
    for &b in chunk_type.iter().chain(data.iter()) {
        let idx = ((crc as u8) ^ b) as usize;
        crc = crc_table[idx] ^ (crc >> 8);
    }
    !crc
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x78);
    output.push(0x01);

    let mut pos = 0;
    while pos < data.len() {
        let remaining = data.len() - pos;
        let block_size = remaining.min(65535);
        let is_final = pos + block_size >= data.len();

        output.push(if is_final { 0x01 } else { 0x00 });
        output.extend_from_slice(&(block_size as u16).to_le_bytes());
        output.extend_from_slice(&(!(block_size as u16)).to_le_bytes());
        output.extend_from_slice(&data[pos..pos + block_size]);
        pos += block_size;
    }

    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    output.extend_from_slice(&adler.to_be_bytes());

    output
}
