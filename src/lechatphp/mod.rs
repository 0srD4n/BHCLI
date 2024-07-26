use crate::{
    trim_newline, CAPTCHA_FAILED_SOLVE_ERR, CAPTCHA_USED_ERR, CAPTCHA_WG_ERR, KICKED_ERR, LANG,
    NICKNAME_ERR, REG_ERR, SERVER_DOWN_500_ERR, SERVER_DOWN_ERR, SESSION_RGX, UNKNOWN_ERR,
};
use base64::engine::general_purpose;
use base64::Engine;
use http::StatusCode;
use regex::Regex;
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
    CaptchaFailedSolveErr, // When auto-solver failed to solve the lechatphp built-in captcha
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
            LoginErr::CaptchaFailedSolveErr => CAPTCHA_FAILED_SOLVE_ERR.to_owned(),
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
    sxiv: bool,
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
            // Otherwise, save the captcha on disk and prompt user for answer
            // println!("Captcha image source: {}", captcha_img);
            // let img_decoded = general_purpose::STANDARD.decode(captcha_img.strip_prefix("data:image/gif;base64,").unwrap()).unwrap();
            //
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

            //
            let img = image::load_from_memory(&img_decoded).unwrap();
            let img_buf = image::imageops::resize(
                &img,
                img.width() * 4,
                img.height() * 4,
                image::imageops::FilterType::Nearest,
            );
            // Save captcha as file on disk
            img_buf.save("captcha.gif").unwrap();

            if sxiv {
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
            } else {
                termage::display_image("captcha.gif", img.width(), img.height());

                // Enter captcha
                print!("captcha: ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut captcha_input).unwrap();
                trim_newline(&mut captcha_input);
            }
        } else {
            captcha_input =
                captcha::solve_b64(captcha_img).ok_or(LoginErr::CaptchaFailedSolveErr)?;
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
            panic!("failed to get view iframe");
        }
    };
    let iframe_src = iframe.attr("src").unwrap();

    let session_captures = SESSION_RGX.captures(iframe_src).unwrap();
    let session = session_captures.get(1).unwrap().as_str();
    Ok(session.to_owned())
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
