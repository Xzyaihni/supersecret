use std::{
    io,
    env,
    process,
    fs::{self, File},
    path::Path,
    str::FromStr
};

use png::{BitDepth, ColorType};


fn crash_msg(message: &str) -> !
{
    println!("{message}");
    process::exit(1)
}

struct Image
{
    data: Vec<u8>,
    img_data: png::OutputInfo,
    bpp: u8,
    has_alpha: bool
}

fn read_image(path: &str) -> Image
{
    let file = File::open(path).unwrap_or_else(|err|
    {
        let msg = format!("error opening image: {err}");
        crash_msg(&msg)
    });

    let png_decoder = png::Decoder::new(&file);
    let mut png_reader = png_decoder.read_info().unwrap_or_else(|err|
    {
        let msg = format!("something wrong with ur image file ; -; ({err})");
        crash_msg(&msg)
    });
    
    let buf_size = png_reader.output_buffer_size();
    let mut buffer = vec![0u8; buf_size];
    
    let img_data = png_reader.next_frame(&mut buffer).unwrap_or_else(|err|
    {
        let msg = format!("something wrong with ur image file ; -; ({err})");
        crash_msg(&msg)
    });

    match img_data.bit_depth
    {
        BitDepth::Four | BitDepth::Two | BitDepth::One =>
        {
            crash_msg("image bit depth is less than 8 ; -; its ova")
        },
        _ => ()
    }

    let (has_alpha, bpp) = match img_data.color_type
    {
        ColorType::Grayscale => (false, 1),
        ColorType::Rgb => (false, 3),
        ColorType::Indexed => crash_msg("cant transfer data using indexed colortypes"),
        ColorType::GrayscaleAlpha => (true, 1),
        ColorType::Rgba => (true, 3)
    };

    Image{
        data: buffer,
        img_data,
        bpp,
        has_alpha
    }
}

#[derive(PartialEq)]
enum Mode
{
    Encode,
    Decode,
    EncodePath
}

impl FromStr for Mode
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err>
    {
        match s
        {
            "encode" => Ok(Mode::Encode),
            "decode" => Ok(Mode::Decode),
            "encode_path" => Ok(Mode::EncodePath),
            _ => Err(())
        }
    }
}

struct BitGetter<'a>
{
    data: &'a [u8],
    byte_ptr: usize,
    bit_ptr: u8
}

impl<'a> BitGetter<'a>
{
    pub fn new(data: &'a [u8]) -> Self
    {
        BitGetter{data, byte_ptr: 0, bit_ptr: 0}
    }
}

impl<'a> Iterator for BitGetter<'a>
{
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.byte_ptr < self.data.len()
        {
            let byte = self.data[self.byte_ptr];
            let bit = (byte >> self.bit_ptr)&1 != 0;

            self.bit_ptr += 1;
            if self.bit_ptr==8
            {
                self.bit_ptr = 0;
                self.byte_ptr += 1;
            }

            Some(bit)
        } else
        {
            None
        }
    }
}

struct BitSetter
{
    data: Vec<u8>,
    current_byte: u8,
    bit_ptr: u8
}

impl BitSetter
{
    pub fn new() -> Self
    {
        BitSetter{data: Vec::new(), current_byte: 0, bit_ptr: 0}
    }

    pub fn push_bit(&mut self, bit: bool)
    {
        let bit = bit as u8;
        self.current_byte = (self.current_byte>>1)|(bit<<7);

        self.bit_ptr += 1;

        if self.bit_ptr==8
        {
            self.data.push(self.current_byte);

            self.current_byte = 0;
            self.bit_ptr = 0;
        }
    }

    pub fn data(&self) -> &[u8]
    {
        &self.data
    }
}

fn main()
{
    let mut args = env::args().skip(1);

    let ok_else_say = |val: Option<String>, message: &str| -> String
    {
        if let Some(val) = val
        {
            val
        } else
        {
            crash_msg(message)
        }
    };

    let mode = ok_else_say(args.next(),
        "plz giv the mode as first arg!! do i decode or encode? ; -;")
        .to_lowercase();

    let mode: Mode = mode.parse().unwrap_or_else(|_|
    {
        let msg = format!("i duno wut mode is {mode} ; -;");
        crash_msg(&msg)
    });

    let image_path = ok_else_say(args.next(),
        "can u plz giv path to the image as second arg? >~<");

    let mut img = read_image(&image_path);
    
    let max_data_bytes =
    {
        let width = img.img_data.width as usize;
        let height = img.img_data.height as usize;
        (width * height * img.bpp as usize) / 8
    };

    let bpp = img.bpp as usize;
    let abpp = bpp + 1;

    match mode
    {
        Mode::Encode | Mode::EncodePath =>
        {
            let (extension, input_message) = if mode == Mode::Encode
            {
                println!("wut would u like to encode? :3");

                let mut input_message = String::new();
                io::stdin().read_line(&mut input_message).unwrap();

                (String::new(), input_message.trim_end().as_bytes().to_vec())
            } else
            {
                let path = ok_else_say(args.next(),
                    "plz provide a third argument for the path!!");

                let path = Path::new(&path);

                let extension = path.extension().map(|s|
                {
                    s.to_os_string().into_string()
                }).unwrap_or(Ok(String::new()))
                    .unwrap_or_else(|_| crash_msg("invalid extension ; -;"));

                (extension, fs::read(path).unwrap_or_else(|err|
                {
                    let msg = format!("something wrong with ur file ; -; ({err})");
                    crash_msg(&msg)
                }))
            };

            //8 bytes for length u64
            //add bytes for extension
            if (input_message.len() + 8 + extension.len()+1)>max_data_bytes
            {
                let real_size = max_data_bytes.saturating_sub(8)
                    .saturating_sub(extension.len())
                    .saturating_sub(1);

                if real_size==0
                {
                    crash_msg("sowy but ur img cant fit anything inside of it.. ; -;")
                }

                let mut magnitude = 1;
                let mut closest = real_size as f64 / 1024.0;
                while closest>=1024.0
                {
                    closest = closest / 1024.0;
                    magnitude += 1;
                }

                let unit = match magnitude
                {
                    1 => "KiB",
                    2 => "MiB",
                    3 => "GiB",
                    4 => "TiB",
                    _ => crash_msg("ur filesize is insane >-<")
                };
                let formatted = format!("{closest:.2} {unit}");

                println!("the maximum msg size that can fit is: {formatted} ({real_size} bytes)");

                crash_msg("i cant put ur message into this img file, try a bigger one? ; ;")
            }

            let len_bytes = input_message.len().to_le_bytes();
            let extension_bytes = [extension.as_bytes(), &[0]].concat();

            let input_message = [&len_bytes, &extension_bytes[..], &input_message[..]].concat();
            let mut bit_getter = BitGetter::new(&input_message);

            for (i, value) in img.data.iter_mut().enumerate()
            {
                if img.has_alpha && (i % abpp == bpp)
                {
                    continue;
                }

                if let Some(bit) = bit_getter.next()
                {
                    if bit
                    {
                        if *value == 255
                        {
                            *value -= 1;
                        } else
                        {
                            *value += 1;
                        }
                    }
                } else
                {
                    break;
                }
            }

            let path = Path::new("./encoded.png");

            let output_file = File::create(path).unwrap_or_else(|err|
            {
                let msg = format!("couldnt create the file ; -; ({err})");
                crash_msg(&msg)
            });

            let mut encoder = png::Encoder::new(
                output_file,
                img.img_data.width,
                img.img_data.height
            );
            encoder.set_color(img.img_data.color_type);
            encoder.set_depth(img.img_data.bit_depth);

            let mut writer = encoder.write_header().unwrap_or_else(|err|
            {
                let msg = format!("i couldnt write the png header ; -; ({err})");
                crash_msg(&msg)
            });

            writer.write_image_data(&img.data).unwrap_or_else(|err|
            {
                let msg = format!("i couldnt write the png body...,, ({err})");
                crash_msg(&msg)
            });
        },
        Mode::Decode =>
        {
            let output_img_path = 
                ok_else_say(args.next(), "plz giv original image path as third arg >-<");

            let original_img = read_image(&output_img_path);

            if original_img.img_data != img.img_data
            {
                crash_msg("those images r completely different ; -;")
            }

            let mut bit_setter = BitSetter::new();

            let iter = original_img.data.iter().zip(img.data.iter());
            for (i, (original_pixel, pixel)) in iter.enumerate()
            {
                if img.has_alpha && (i % abpp == bpp)
                {
                    continue;
                }

                let bit = original_pixel != pixel;

                bit_setter.push_bit(bit);
            }

            let length_buffer: [u8; 8] = bit_setter.data()[0..8].try_into()
                .unwrap_or_else(|err|
                {
                    let msg = format!("cant read length ; -; ({err})");
                    crash_msg(&msg)
                });

            let length = u64::from_le_bytes(length_buffer);
            let extension_end = 8 + bit_setter.data()[8..].iter().position(|x| *x==0)
                .unwrap_or_else(|| crash_msg("cant find the extension.. ; -;"));
            let extension = String::from_utf8_lossy(&bit_setter.data()[8..extension_end]);

            let data_start = extension_end+1;
            let data = &bit_setter.data()[data_start..(data_start + length as usize)];
            if extension.is_empty()
            {
                let message = String::from_utf8_lossy(data);
                println!("plaintext message: {message}");
            } else
            {
                let mut filename = String::from("decoded.");
                filename += &extension;

                let path = Path::new(&filename);
                fs::write(path, &data).unwrap_or_else(|err|
                {
                    let msg = format!("couldnt save the decoded file ; -; ({err})");
                    crash_msg(&msg)
                });

                println!("file message written to: {}", path.display());
            }
        }
    }
}


#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn bit_getter_worky()
    {
        let test_data = [123, 255, 52];

        let mut bit_getter = BitGetter::new(&test_data);

        //123
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(false));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(false));

        //255
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));

        //52
        assert_eq!(bit_getter.next(), Some(false));
        assert_eq!(bit_getter.next(), Some(false));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(false));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(true));
        assert_eq!(bit_getter.next(), Some(false));
        assert_eq!(bit_getter.next(), Some(false));

        assert_eq!(bit_getter.next(), None);
        
    }
}
