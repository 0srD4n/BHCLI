use crate::{
    trim_newline, CAPTCHA_USED_ERR, CAPTCHA_WG_ERR, KICKED_ERR, LANG,
    NICKNAME_ERR, REG_ERR, SERVER_DOWN_500_ERR, SERVER_DOWN_ERR, SESSION_RGX, UNKNOWN_ERR,
};
use base64::engine::general_purpose;
use base64::Engine;
use http::StatusCode;
use regex::Regex;
use image::DynamicImage;

use reqwest::blocking::Client;
use select::document::Document;
use select::predicate::{And, Attr, Name};
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{error, fs, io, thread};

pub mod captcha;

#[derive(Debug)]
pub enum LoginErr {
    ServerDownErr,
    ServerDown500Err,
    CaptchaUsedErr,
    CaptchaWgErr,
    RegErr,
    NicknameErr,
    KickedErr,
    UnknownErr,
    Reqwest(reqwest::Error),
}

impl From<reqwest::Error> for LoginErr {
    fn from(value: reqwest::Error) -> Self {
        LoginErr::Reqwest(value)
    }
}

impl Display for LoginErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LoginErr::ServerDownErr => SERVER_DOWN_ERR.to_owned(),
            LoginErr::ServerDown500Err => SERVER_DOWN_500_ERR.to_owned(),
            LoginErr::CaptchaUsedErr => CAPTCHA_USED_ERR.to_owned(),
            LoginErr::CaptchaWgErr => CAPTCHA_WG_ERR.to_owned(),
            LoginErr::RegErr => REG_ERR.to_owned(),
            LoginErr::NicknameErr => NICKNAME_ERR.to_owned(),
            LoginErr::KickedErr => KICKED_ERR.to_owned(),
            LoginErr::UnknownErr => UNKNOWN_ERR.to_owned(),
            LoginErr::Reqwest(e) => e.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl error::Error for LoginErr {}

pub fn login(
    client: &Client,
    base_url: &str,
    page_php: &str,
    username: &str,
    password: &str,
    color: &str,
    manual_captcha: bool,
) -> Result<String, LoginErr> {
    // Get login page
    let login_url = format!("{}/{}", &base_url, &page_php);
    let resp = client.get(&login_url).send()?;
    if resp.status() == StatusCode::BAD_GATEWAY {
        return Err(LoginErr::ServerDownErr);
    }
    let resp = resp.text()?;
    let doc = Document::from(resp.as_str());

    // Post login form
    let mut params = vec![
        ("action", "login".to_owned()),
        ("lang", LANG.to_owned()),
        ("nick", username.to_owned()),
        ("pass", password.to_owned()),
        ("colour", color.to_owned()),
    ];

    if let Some(captcha_node) = doc
        .find(And(Name("input"), Attr("name", "challenge")))
        .next()
    {
        let captcha_value = captcha_node.attr("value").unwrap();
        let captcha_img = doc.find(Name("img")).next().unwrap().attr("src").unwrap();

        let mut captcha_input = String::new();
        if manual_captcha {
            // Attempt to strip the appropriate prefix based on the MIME type
            let base64_str =
                if let Some(base64) = captcha_img.strip_prefix("data:image/png;base64,") {
                    base64
                } else if let Some(base64) = captcha_img.strip_prefix("data:image/gif;base64,") {
                    base64
                } else {
                    panic!("Unexpected captcha image format. Expected PNG or GIF.");
                };

            // Decode the base64 string into binary image data
            let img_decoded = general_purpose::STANDARD.decode(base64_str).unwrap();

            let img = image::load_from_memory(&img_decoded).unwrap();
            let img_buf = image::imageops::resize(
                &img,
                img.width() * 4,
                img.height() * 4,
                image::imageops::FilterType::Nearest,
            );
            // Save captcha as file on disk
            img_buf.save("captcha.gif").unwrap();

            // Pilihan untuk menyelesaikan captcha
            println!("choice your methode captcha view:");
            println!("1. Terminal (ASCII)");
            println!("2. Sxiv");
            print!("choice option: ");
            io::stdout().flush().unwrap();
            let mut choice = String::new();
            io::stdin().read_line(&mut choice).unwrap();
            let choice = choice.trim();

            match choice {
                "1" => {
                    // Menampilkan captcha di terminal secara langsung (ASCII)
                    println!("Captcha:");
                    let img_ascii = image_to_ascii(&img, 80, 40);
                    println!("{}", img_ascii);
                    
                    // Prompt untuk memasukkan captcha
                    print!("Masukkan captcha: ");
                    io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut captcha_input).unwrap();
                    trim_newline(&mut captcha_input);
                },
                "2" => {
                    let mut sxiv_process = Command::new("sxiv")
                        .arg("captcha.gif")
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .expect("Failed to open image with sxiv");

                    // Prompt the user to enter the CAPTCHA
                    print!("Please enter the CAPTCHA: ");
                    io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut captcha_input).unwrap();
                    trim_newline(&mut captcha_input);

                    // Close the sxiv window
                    sxiv_process.kill().expect("Failed to close sxiv");

                    println!("Captcha input: {}", captcha_input);
                },
                _ => {
                    println!("Pilihan tidak valid. Menggunakan metode terminal (ASCII).");
                    // Menampilkan captcha di terminal secara langsung
                    println!("Captcha:");
                    let img_ascii = image_to_ascii(&img, 80, 40);
                    println!("{}", img_ascii);
                    
                    print!("Masukkan captcha: ");
                    io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut captcha_input).unwrap();
                    trim_newline(&mut captcha_input);
                }
            }

            // Menyimpan captcha yang baru dimasukkan
        } else {
            // Menyelesaikan captcha secara otomatis
            captcha_input = match captcha::solve_b64(captcha_img) {
                Some(solved_captcha) => solved_captcha,
                None => {
                    // Jika gagal menyelesaikan captcha secara otomatis, coba gunakan captcha sebelumnya
                    if let Ok(previous_captcha) = fs::read_to_string("previous_captcha.txt") {
                        println!("Menggunakan captcha sebelumnya: {}", previous_captcha.trim());
                        previous_captcha.trim().to_string()
                    } else {
                        // Jika tidak ada captcha sebelumnya, gunakan metode manual
                        println!("Gagal menyelesaikan captcha secara otomatis dan tidak ada captcha sebelumnya. Menggunakan metode manual.");
                        let img = image::load_from_memory(&general_purpose::STANDARD.decode(captcha_img.split(',').last().unwrap()).unwrap()).unwrap();
                        println!("Captcha:");
                        let img_ascii = image_to_ascii(&img, 80, 40);
                        println!("{}", img_ascii);
                        
                        print!("Masukkan captcha: ");
                        io::stdout().flush().unwrap();
                        let mut manual_input = String::new();
                        io::stdin().read_line(&mut manual_input).unwrap();
                        trim_newline(&mut manual_input);
                        manual_input
                    }
                }
            };
        }

        params.extend(vec![
            ("challenge", captcha_value.to_owned()),
            ("captcha", captcha_input.clone()),
        ]);
    }

    let mut resp = client.post(&login_url).form(&params).send()?;
    match resp.status() {
        StatusCode::BAD_GATEWAY => return Err(LoginErr::ServerDownErr),
        StatusCode::INTERNAL_SERVER_ERROR => return Err(LoginErr::ServerDown500Err),
        _ => {}
    }

    let mut refresh_header = resp
        .headers()
        .get("refresh")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    while refresh_header != "" {
        let rgx = Regex::new(r#"URL=(.+)"#).unwrap();
        let refresh_url = format!(
            "{}{}",
            base_url,
            rgx.captures(&refresh_header)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
        );
        println!("waitroom enabled, wait 10sec");
        thread::sleep(Duration::from_secs(10));
        resp = client.get(refresh_url.clone()).send()?;
        refresh_header = resp
            .headers()
            .get("refresh")
            .map(|v| v.to_str().unwrap())
            .unwrap_or("");
    }

    let mut resp = resp.text()?;
    if resp.contains(CAPTCHA_USED_ERR) {
        return Err(LoginErr::CaptchaUsedErr);
    } else if resp.contains(CAPTCHA_WG_ERR) {
        return Err(LoginErr::CaptchaWgErr);
    } else if resp.contains(REG_ERR) {
        return Err(LoginErr::RegErr);
    } else if resp.contains(NICKNAME_ERR) {
        return Err(LoginErr::NicknameErr);
    } else if resp.contains(KICKED_ERR) {
        return Err(LoginErr::KickedErr);
    }

    let mut doc = Document::from(resp.as_str());
    if let Some(body) = doc.find(Name("body")).next() {
        if let Some(body_class) = body.attr("class") {
            if body_class == "error" {
                if let Some(h2) = doc.find(Name("h2")).next() {
                    log::error!("{}", h2.text());
                }
                return Err(LoginErr::UnknownErr);
            } else if body_class == "failednotice" {
                log::error!("failed logins: {}", body.text());
                let nc = doc.find(Attr("name", "nc")).next().unwrap();
                let nc_value = nc.attr("value").unwrap().to_owned();
                let params: Vec<(&str, String)> = vec![
                    ("lang", LANG.to_owned()),
                    ("nc", nc_value.to_owned()),
                    ("action", "login".to_owned()),
                ];
                resp = client.post(&login_url).form(&params).send()?.text()?;
                doc = Document::from(resp.as_str());
            }
        }
    }

    let iframe = match doc.find(Attr("name", "view")).next() {
        Some(view) => view,
        None => {
            fs::write("./dump_login_err.html", resp.as_str()).unwrap();
            return Err(LoginErr::UnknownErr); // Ubah panic menjadi return Err
        }
    };
    let iframe_src = iframe.attr("src").unwrap();

    let session_captures = SESSION_RGX.captures(iframe_src).unwrap();
    let session = session_captures.get(1).unwrap().as_str();
    Ok(session.to_owned())
}

// Fungsi untuk mengubah gambar menjadi ASCII art
fn image_to_ascii(img: &DynamicImage, width: u32, height: u32) -> String {
    let img = img.resize_exact(width, height, image::imageops::FilterType::Nearest);
    let img = img.to_luma8();
    let mut result = String::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let intensity = pixel[0];
            let ascii_char = match intensity {
                0..=63 => '#',
                64..=127 => '+',
                128..=191 => '-',
                192..=255 => '.',
            };
            result.push(ascii_char);
        }
        result.push('\n');
    }
    result
}


pub fn logout(
    client: &Client,
    base_url: &str,
    page_php: &str,
    session: &str,
) -> anyhow::Result<()> {
    let full_url = format!("{}/{}", &base_url, &page_php);
    let params = [("action", "logout"), ("session", &session), ("lang", LANG)];
    client.post(&full_url).form(&params).send()?;
    Ok(())
}