use crossterm::queue;
use image::{DynamicImage, ImageBuffer, Rgb};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize, Serializer};

// A textel is like a pixel but made up of character
// The are the individual elements comprising 
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Textel {
    char: char,
    color: [u8; 3],
}

// A generic AsciiArt with parameters for its width and height in textels
// These need to be serialized so that they can be saved with the record collection data, though I
// should've just saved them as a string of data instead of imiplementing the serde traits
#[derive(Debug)]
pub struct AsciiArt<const WIDTH: usize, const HEIGHT: usize> {
    data: [[Textel; WIDTH]; HEIGHT],
}

impl<const WIDTH: usize, const HEIGHT: usize> AsciiArt<WIDTH, HEIGHT> {
    // Converts image to AsciiArt
    pub fn from_image(image: DynamicImage) -> std::io::Result<AsciiArt<WIDTH, HEIGHT>> {
        // Converts image to Jpeg like data (i.e. no alpha channel)
        let image = image.into_rgb8();
        let (img_w, img_h) = (image.width(), image.height());
        let pix_tex_ratio = (img_w as usize / WIDTH, img_h as usize / HEIGHT);

        let mut art = blank_art::<WIDTH, HEIGHT>();

        // Gets textel by sampling the image data 
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                art.data[y][x] = sample_at(x, y, pix_tex_ratio, &image);
            }
        }

        Ok(art)
    }

    // Prints the AsciiArt like text
    pub fn print(&self) -> std::io::Result<()> {
        use std::io::{stdout, Write};
        let mut stdout = stdout();

        use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};

        // Setting the background color isn't necessary as we are only printing '█' characters 
        // which don't show the background
        queue!(stdout, SetBackgroundColor(Color::White))?;
        // Prints each individual textel according to their character and color
        for row in self.data.iter() {
            for textle in row.iter() {
                let color = Color::Rgb {
                    r: textle.color[0],
                    g: textle.color[1],
                    b: textle.color[2],
                };
                queue!(
                    stdout,
                    SetForegroundColor(color),
                    Print(textle.char.to_string())
                )?;
            }
            queue!(stdout, ResetColor, Print("\n"))?;
        }
        stdout.flush()?;

        Ok(())
    }

    // Prints the AsciiArt at a given terminal position
    // Is essentially the same function as print(), but moving the cursor accordingly
    pub fn print_at(&self, position: (u16, u16)) -> std::io::Result<()> {
        use std::io::{stdout, Write};
        let mut stdout = stdout();

        use crossterm::cursor;
        use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};

        queue!(stdout, SetBackgroundColor(Color::White))?;
        for (i, row) in self.data.iter().enumerate() {
            queue!(stdout, cursor::MoveTo(position.0, position.1 + i as u16))?;
            for textle in row.iter() {
                let color = Color::Rgb {
                    r: textle.color[0],
                    g: textle.color[1],
                    b: textle.color[2],
                };
                queue!(
                    stdout,
                    SetForegroundColor(color),
                    Print(textle.char.to_string())
                )?;
            }
            queue!(stdout, ResetColor)?;
        }
        stdout.flush()?;

        Ok(())
    }
}

// Samples the image for a textel at a given positon, taking the average of 9 color samples
fn sample_at(
    tex_x: usize,
    tex_y: usize,
    pix_tex_ratio: (usize, usize),
    image: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Textel {
    // The top-left corner of the rectangle being sampled for the given textel
    let base = (tex_x * pix_tex_ratio.0, tex_y * pix_tex_ratio.1);
    // The offsets of the sample points for the textel
    // This is pretty much just a 3x3 grid
    let offsets: [[(usize, usize); 3]; 3] = [
        [
            (0, 0),
            ((pix_tex_ratio.0 - 1) / 2, 0),
            (pix_tex_ratio.0 - 1, 0),
        ],
        [
            (0, ((pix_tex_ratio.1 - 1) / 2)),
            ((pix_tex_ratio.0 - 1) / 2, ((pix_tex_ratio.1 - 1) / 2)),
            (pix_tex_ratio.0 - 1, ((pix_tex_ratio.1 - 1) / 2)),
        ],
        [
            (0, pix_tex_ratio.1 - 1),
            ((pix_tex_ratio.0 - 1) / 2, pix_tex_ratio.1 - 1),
            (pix_tex_ratio.0 - 1, pix_tex_ratio.1 - 1),
        ],
    ];

    // Retrieves the colors at the sample positions described just above
    let samples = offsets
        .map(|ls| ls.map(|(dx, dy)| image.get_pixel((base.0 + dx) as u32, (base.1 + dy) as u32)));

    // Sums the samples' color
    let mut sum: [u32; 3] = [0, 0, 0];
    for row in samples.iter() {
        for color in row.iter() {
            sum[0] += color[0] as u32;
            sum[1] += color[1] as u32;
            sum[2] += color[2] as u32;
        }
    }
    // To then take the average of the 9 samples
    let color = sum.map(|x| (x / 9) as u8);

    Textel { char: '█', color }
}

// A utility function that returns a blank AsciiArt struct 
pub fn blank_art<const WIDTH: usize, const HEIGHT: usize>() -> AsciiArt<WIDTH, HEIGHT> {
    AsciiArt {
        data: [[Textel {
            char: ' ',
            color: [0, 0, 0],
        }; WIDTH]; HEIGHT],
    }
}

// The following code is needed to serialize the AsciiArt so that it can be serialized along with
// the rest of the record data. This is not a great implementation and should be changed!

// A wrapper struct needed since we can't impl a base type [Textel; WIDTH]
// I used the crate "serde_arrays" to automatically implement the Serialize and Deserialize traits
// for a row of Textels. This crate should in principle also work for 2D arrays such as AsciiArt's
// data, but for some reason it wouldn't work for me so I ended up implementing manually.
#[derive(Serialize, Deserialize)]
pub struct RowWrapper<const WIDTH: usize> {
    #[serde(with = "serde_arrays")]
    row: [Textel; WIDTH],
}

// Implements Serialize trait for AsciiArt. This is made easy since "serde_array" handles
// serializing each row.
impl<const WIDTH: usize, const HEIGHT: usize> Serialize for AsciiArt<WIDTH, HEIGHT> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple(HEIGHT)?;
        for row in self.data {
            let wrapper = RowWrapper { row };
            s.serialize_element(&wrapper)?;
        }
        s.end()
    }
}

// The code to implement the Deserialize trait for AsciiArt
use serde::de::{SeqAccess, Visitor};
use std::fmt;

struct AsciiArtVisitor<const WIDTH: usize, const HEIGHT: usize> {}
impl<const WIDTH: usize, const HEIGHT: usize> AsciiArtVisitor<WIDTH, HEIGHT> {
    fn new() -> Self {
        AsciiArtVisitor {}
    }
}

impl<'de, const WIDTH: usize, const HEIGHT: usize> Visitor<'de> for AsciiArtVisitor<WIDTH, HEIGHT> {
    type Value = AsciiArt<WIDTH, HEIGHT>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a AsciiArt struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut art = blank_art::<WIDTH, HEIGHT>();
        let mut i: usize = 0;
        loop {
            match seq.next_element::<RowWrapper<WIDTH>>()? {
                Some(row) => {
                    art.data[i] = row.row;
                    i += 1;
                }
                None => break,
            }
        }

        Ok(art)
    }
}

impl<'de, const WIDTH: usize, const HEIGHT: usize> Deserialize<'de> for AsciiArt<WIDTH, HEIGHT> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(AsciiArtVisitor::new())
    }
}


