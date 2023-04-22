#![allow(dead_code)]
#![allow(unused)]

struct Header{
    magic: [u8; 4],
    w: u32,
    h: u32,
    channels: u8,
    colorspace: u8
}

impl Header {
    pub fn new(w: u32, h: u32, channels: u8) -> Self {
        Self {
            magic: *b"qoif",
            w: w,
            h: h,
            channels: channels,
            colorspace: 0
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.magic);
        bytes.extend(self.w.to_be_bytes());
        bytes.extend(self.h.to_be_bytes());
        bytes.push(self.channels);
        bytes.push(self.colorspace);

        return bytes;
    }
}

#[repr(u8)]
enum QoiOp {
    RGBA = 0xFF,
    RGB = 0xFE,
    Run = 0x03<<6,
    Luma = 0x02<<6,
    Diff = 0x01<<6,
    Index = 0x00
}

struct PixelDiff {
    r: u8,
    g: u8,
    b: u8
}


#[derive(Copy, Clone)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

impl Pixel{
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r,
            g: g,
            b: b,
            a: a
        }
    }

    pub fn to_array(&self, channels: u8) -> Vec<u8> {
        let mut arr = Vec::with_capacity( channels.into() );
        arr.push(self.r);
        arr.push(self.g);
        arr.push(self.b);
        if channels == 4 {
            arr.push(self.a);
        }
        return arr;
    }

    pub fn from_array(arr: &[u8], channels: u8) -> Self {
        let mut p = Pixel::new(arr[0], arr[1], arr[2], 0);

        if channels == 4 {
            p.a = arr[3];
        }

        return p;
    }

    pub fn equals(&self, other: &Pixel, channels: u8) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b && (channels == 3 || self.a == other.a)
    }

    pub fn hash(&self) -> u8 {
        let r = self.r as u32 * 3;
        let g = self.g as u32 * 5;
        let b = self.b as u32 * 7;
        let a = self.a as u32 * 11;
        return ((r + g + b + a) & 63 ) as u8;
    }

    pub fn diff(&self, prev: Pixel) -> Option<PixelDiff> {

        let diff = PixelDiff {
            r: self.r.wrapping_sub(prev.r).wrapping_add(2),
            g: self.g.wrapping_sub(prev.g).wrapping_add(2),
            b: self.b.wrapping_sub(prev.b).wrapping_add(2)
        };

        let bl = diff.r < 4 && diff.g < 4 && diff.b < 4;
        if bl {
            return Some(diff);
        }
        return None;
    }

    pub fn luma(&self, prev: Pixel) -> Option<PixelDiff> {
        let diff_g: i16 = self.g as i16 - prev.g as i16;

        if diff_g >= -32 && diff_g <= 31 {
            let dg = (diff_g + 32) as u8;

            let diff_rg = self.r.wrapping_sub(prev.r).wrapping_sub(dg).wrapping_sub(32);
            let diff_bg = self.b.wrapping_sub(prev.b).wrapping_sub(dg).wrapping_sub(32);

            diff_rg.wrapping_add(8);
            diff_bg.wrapping_add(8);

            if diff_rg < 16 && diff_bg < 16 {
                return Some(PixelDiff{
                    r: diff_rg,
                    g: dg,
                    b: diff_bg
                });
            }
        }
        return None;
    }

    pub fn copy(&self) -> Self{
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a
        }
    }
}


fn write_runlength(output: &mut Vec<u8>, count: u8){
    let b = (QoiOp::Run as u8) | (count - 1);
    output.push(b);
}

fn write_index(output: &mut Vec<u8>, index: u8){
    let b = (QoiOp::Index as u8) | index ;
    output.push(b);
}

fn write_diff(output: &mut Vec<u8>, diff: &PixelDiff){
    let r = diff.r<<4;
    let g = diff.g<<2;
    let b = diff.b;
    let byte = (QoiOp::Diff as u8) | r | g | b;

    output.push(byte);
}

fn write_luma(output: &mut Vec<u8>, luma: &PixelDiff){
    let mut bytes: [u8; 2] = [
        QoiOp::Luma as u8 | luma.g,
        (luma.r << 4) | luma.b
    ];

    // println!("Got luma:");
    // println!("diff g: {}", luma.g as i16 - 32);
    // println!("dr - dg: {}", luma.r as i16 - 8);
    // println!("db - dg: {}", luma.b as i16 - 8);

    output.extend(bytes);
}



fn compress_image(input: &[u8], output: &mut Vec<u8>, settings: &Header) {

    output.reserve(input.len() / 4);

    let mut indexed_pixels = [Pixel::new(0, 0, 0, 0); 64];
    let mut iter = input.chunks_exact( settings.channels.into() );

    let mut prev_pixel = Pixel::new(0, 0, 0, 255);
    let mut run_length = 0;
    let channels = settings.channels;


    loop {
        let next_val = match iter.next() {
            None => break,
            Some(val) => val
        };
        let cur_pixel = Pixel::from_array(next_val, channels);
        let index = cur_pixel.hash();


        if cur_pixel.equals(&prev_pixel, channels) {
            run_length += 1;
            if run_length == 62 {
                write_runlength(output, run_length);
                run_length = 0;
            }
        }
        else{

            if run_length != 0 {
                write_runlength(output, run_length);
                run_length = 0;
            }

            let diff = cur_pixel.diff(prev_pixel);
            let luma = cur_pixel.luma(prev_pixel);

            if cur_pixel.equals(&indexed_pixels[index as usize], channels) {
                write_index(output, index);
            }
            else if !diff.is_none() {
                write_diff(output, &diff.unwrap());
            }
            else if !luma.is_none() {
                write_luma(output, &luma.unwrap());
            }
            else{
                let mask = match channels {
                    3 => QoiOp::RGB,
                    4 => QoiOp::RGBA,
                    _ => panic!("Invalid channel count")
                };
                output.push(mask as u8);
                output.extend( cur_pixel.to_array(channels) );
            }
            prev_pixel = cur_pixel.copy();

        }
        indexed_pixels[index as usize] = cur_pixel.copy();
    }

    if run_length >= 1 {
        write_runlength(output, run_length);
    }

    let end_array: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
    output.extend(end_array);
}




pub fn compress(input: &[u8], output: &mut Vec<u8>, w: u32, h: u32, channels: u8){
    let header = Header::new(w, h, channels);

    output.append( &mut header.to_bytes() );
    compress_image(input, output, &header);

}

