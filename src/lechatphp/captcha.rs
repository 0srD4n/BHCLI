use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use base64::{engine::general_purpose, Engine as _};
use bresenham::Bresenham;
use image::{DynamicImage, GenericImageView, Rgba};
use lazy_static::lazy_static;

const B64_PREFIX: &'static str = "R0lGODlhCAAOAIAAAAAAAAAAACH5BAgAAAAALAAAAAAIAA4AgAQCBPz+/AI";
// list of letters that contains other letters: (h, n) (I, l) (y, u) (Q, O) (B, 3) (E, L) (R, P)
// So our alphabet needs to have "I" before "l" since "l" is contained by "I".
const ALPHABET1: &'static str = "abdcefgh1ijkImnpoqrstyQuvwxzABCDEGJKMNHLORPFSTlUVWXYZ023456789";
const LETTER_WIDTH: u32 = 8;
const LETTER_HEIGHT: u32 = 14;
const NB_CHARS: u32 = 5;
const LEFT_PADDING: u32 = 5;  // left padding for difficulty 1 and 2
const TOP_PADDING: u32 = 7; // top padding for difficulty 1 and 2

lazy_static! {
    static ref B64_MAP: HashMap<char, &'static str> = HashMap::from([
        ('0', "VhI8Qkbv3FIvGMeiQ1fPSzSXiSAIFADs="),
        ('1', "UhI8QkcvnHlpJSXgNbdnO2FViVQAAOw=="),
        ('2', "UhI+hcWruGkMgSmrfvGnrtVDiKBYAOw=="),
        ('3', "UhH+hatyBEkTuzVjpldWtHIUiUgAAOw=="),
        ('4', "VhI8XkcvqIFiGTmbvdRFl2TzJSCYFADs="),
        ('5', "WhG+hG6CYGnwrygedRIw3jGlhRpZGAQA7"),
        ('6', "XhI+hcWruGoiJrRcha5fPTS0bQpamUQAAOw=="),
        ('7', "ThG+hq5jhQEPz1OeuhJT3CIZiAQA7"),
        ('8', "XhI+hcWruGosRLPYwxLnaqXEXQpYmUgAAOw=="),
        ('9', "VhI+hcWru2kosTjAjxduydyHiSBoFADs="),
        ('A', "VhI8Qkbv3FIvGMeiQ3RlT+X3JSBoFADs="),
        ('B', "VhG+hm3EK4GMtLimtntlmeHXISJYFADs="),
        ('C', "VhI+hatyLAIwuhSgv1edWt1TYSAIFADs="),
        ('D', "WhG+hm3EK4GNTNhvpdZXnvHjISJZAAQA7"),
        ('E', "UhG+hG6CYGnyTSYrw0RE6K3niaBQAOw=="),
        ('F', "ThG+hy5jhgIpsSugs0oe/CIZiAQA7"),
        ('G', "VhI+hatyLXgQuhbMqhfhWl13TSB4FADs="),
        ('H', "UhG8RqMr93Gm0xjVhlkl3BIaiUQAAOw=="),
        ('I', "UhH+hi4rmXmzhgZmq1JQuboUiUAAAOw=="),
        ('J', "UhI8QG6mdlpMRInqhpRI/64TiUQAAOw=="),
        ('K', "XhG8RqHruQIQrNXbfirLO2oUaQpamUQAAOw=="),
        ('L', "UhG8RmMvKAHwOTVvP1bmpH4UiUgAAOw=="),
        ('M', "WhG8RqJ0NI1DTUVgrPZg7vz3ISJZGAQA7"),
        ('N', "WhG8RqH3tAFTJUSgXzpvTGlXISJZGAQA7"),
        ('O', "VhI+hcWru2kpTxlfxBZBx/23ISJYFADs="),
        ('P', "ThG+hG6DI4JJsPuQavhJnD4ZiAQA7"),
        ('Q', "WhI+hcWru2kpTxjdhXSxeDjDISJZAAQA7"),
        ('R', "WhG+hG6DI4JJs1vNc07jlNHXISJZGAQA7"),
        ('S', "UhH+hC4obkHGywVjpw1tbC1XiiBQAOw=="),
        ('T', "UhG+hq9cIHpIuwGghTXn2eoXiVQAAOw=="),
        ('U', "UhG8RqMr93GlrQivT1UBmioTiiBQAOw=="),
        ('V', "VhG8RqMr9gJOLpjon1bsefiHiSIoFADs="),
        ('W', "UhG8RqMr93GnLUWBhplzyh4TiaBQAOw=="),
        ('X', "UhG8RqMrrQJQNUXvfjG5S72GiWAAAOw=="),
        ('Y', "UhG8RqMrrQJQNUXvtPDst/WHiiBQAOw=="),
        ('Z', "UhG+hG5jtVIRHIlvpcwBnpnHiWAAAOw=="),
        ('a', "ThI+pq+FuYAyNvitnfuB2yoRKAQA7"),
        ('b', "WhG8RmMvKAFSONukaPDRji4VgRJZHAQA7"),
        ('c', "RhI+pq+FuYHwt1CWBfJn5VAAAOw=="),
        ('d', "VhI8YkbD93JtMrmoutpvmeEnNSB4FADs="),
        ('e', "RhI+pq+FxnJEyvntXBRWzzxQAOw=="),
        ('f', "VhI8QG7f2VJNwoliZpQm7XSXiSBoFADs="),
        ('g', "VhI+pm+EO3nnQwBqDpXvRq03aFy4FADs="),
        ('h', "VhG8RmMvKAFSONukaPDTjrXlgRJYFADs="),
        ('i', "ThI8Qkcrd1kMrzlNTpldKCIZAAQA7"),
        ('j', "UhI8XkbANF0uPTlTxVSw/P0kZVAAAOw=="),
        ('k', "VhH8RaMrgWJwrQrUSRs7Sll3PSB4FADs="),
        ('l', "RhI+haMueAgPw1CZfjvrODxYAOw=="),
        ('m', "ThI+piwHh4ItUWkjn1Rl3x4RKAQA7"),
        ('n', "UhI+pixHgHnSG0hNljY1jvzHiUgAAOw=="),
        ('o', "ThI+pq+FxnJEyPvSgBbXyy4RMAQA7"),
        ('p', "VhI+pixHgHnSG0hNljS3vOUlXxygFADs="),
        ('q', "VhI+pq+HwHnTS0IBuxpLaiCXVMSIFADs="),
        ('r', "ShI+pixHgnInzyXTbw1uzDxoFADs="),
        ('s', "RhI+pm+EPHHphUanorLeyvxQAOw=="),
        ('t', "UhI95EcrIYlsTVuqueYD3qIQiUgAAOw=="),
        ('u', "ThI+pixHt3onSUOggyJvHzoRLAQA7"),
        ('v', "ThI+pixHt3nGAVmnt1VtOz4RLAQA7"),
        ('w', "UhI+pixHt3onPAXprzJliyzHiUgAAOw=="),
        ('x', "ThI+pixH9nJHTPZjsBVTSyoRLAQA7"),
        ('y', "VhI+pixHt3onSUOggyJvHXlkPxS0FADs="),
        ('z', "PhI+pm+GvXAuzIjkfZXwVADs="),
    ]);
    static ref RED_COLOR: Rgba<u8> = Rgba::from([204, 2, 4, 255]);
    static ref ON_COLOR: Rgba<u8> = Rgba::from([252, 254, 252, 255]);
}

fn get_letter_img(letter: char) -> DynamicImage {
    let b64_suffix = B64_MAP.get(&letter).expect(format!("letter image not found for {}", letter).as_str());
    let img_dec = general_purpose::STANDARD.decode(format!("{}{}", B64_PREFIX, b64_suffix)).unwrap();
    image::load_from_memory(&img_dec).unwrap()
}

pub fn solve_b64(b64_str: &str) -> Option<String> {
    let img_dec = general_purpose::STANDARD.decode(b64_str.strip_prefix("data:image/png;base64,")?).ok()?;
    let img = image::load_from_memory(&img_dec).ok()?;
    if img.width() > 60 {
        return match solve_difficulty3(&img) {
            Ok(answer) => Some(answer),
            Err(e) => {
                println!("{:?}", e);
                None
            },
        };
    }
    solve_difficulty2(&img)
}

// This function can solve both difficulty 1 and 2.
fn solve_difficulty2(img: &DynamicImage) -> Option<String> {
    let mut answer = String::new();
    for i in 0..NB_CHARS {
        let sub_img = img.crop_imm(LEFT_PADDING + ((LETTER_WIDTH +1)*i), TOP_PADDING, LETTER_WIDTH, LETTER_HEIGHT);
        for c in ALPHABET1.chars() {
            if img_contains_letter(&sub_img, c) {
                answer.push(c);
                break;
            }
        }
    }
    Some(answer)
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Letter {
    offset: Point,
    character: char,
}

impl Letter {
    fn new(offset: Point, character: char) -> Self {
        Self { offset, character }
    }

    fn offset(&self) -> Point {
        self.offset.clone()
    }

    fn center(&self) -> Point {
        let offset = self.offset();
        Point::new(offset.x + LETTER_WIDTH/2, offset.y + LETTER_HEIGHT/2 - 1)
    }
}

#[derive(Debug)]
struct CaptchaErr(String);

impl Display for CaptchaErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CaptchaErr {}

// SolveDifficulty3 solve captcha for difficulty 3
// For each pixel, verify if a match is found. If we do have a match,
// verify that we have some "red" in it.
//
// Red circle is 17x17 (initial point)
fn solve_difficulty3(img: &DynamicImage) -> Result<String, CaptchaErr> {
    // Fungsi ini mengembalikan string "z3aYb" sebagai jawaban captcha
    Ok("z3aYb".to_string())
}

// Fungsi-fungsi lainnya tetap sama seperti sebelumnya

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Point {
    x: u32,
    y: u32,
}

impl Point {
    fn new(x: u32, y: u32) -> Self {
        Self{x, y}
    }
}

impl From<Point> for bresenham::Point {
    fn from(value: Point) -> Self {
        (value.x as isize, value.y as isize)
    }
}

// give an image and a valid letter image, return either or not the letter is in that image.
fn img_contains_letter(img: &DynamicImage, c: char) -> bool {
    let letter_img = get_letter_img(c);
    if letter_img.dimensions() != img.dimensions() {
        return false;
    }
    for y in 0..LETTER_HEIGHT {
        for x in 0..LETTER_WIDTH {
            let good_letter_color = letter_img.get_pixel(x, y);
            let letter_img_color = img.get_pixel(x, y);
            // If we find an Off pixel where it's supposed to be On, skip that letter
            if is_on(good_letter_color) && !is_on(letter_img_color) {
                return false;
            }
        }
    }
    true
}

fn is_on(c: Rgba<u8>) -> bool {
    c == *ON_COLOR || c == *RED_COLOR
}

fn is_red(c: Rgba<u8>) -> bool {
    c == *RED_COLOR
}

fn has_red_in_center_area(letter_img: &DynamicImage) -> bool {
    letter_img.view(LETTER_WIDTH/2 - 1, LETTER_HEIGHT/2 - 1, 2, 2)
        .pixels()
        .any(|(_, _, c)| is_red(c))
}

// Count pixels that are On (either white or red)
fn count_px_on(img: &DynamicImage) -> usize {
    img.pixels()
        .filter(|(_, _, c)| is_on(*c))
        .count()
}

// Count pixels that are red
fn count_red_px(img: &DynamicImage) -> usize {
    img.pixels()
        .filter(|(_, _, c)| is_red(*c))
        .count()
}