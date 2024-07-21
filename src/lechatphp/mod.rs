use crate::{
    trim_newline, CAPTCHA_FAILED_SOLVE_ERR, CAPTCHA_USED_ERR, CAPTCHA_WG_ERR, KICKED_ERR, LANG,
    NICKNAME_ERR, REG_ERR, SERVER_DOWN_500_ERR, SERVER_DOWN_ERR, SESSION_RGX, UNKNOWN_ERR,
};
use anyhow::{Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use http::StatusCode;
use serde::de::Unexpected::Other;
use image::DynamicImage;
use regex::Regex;
use reqwest::blocking::{Client, Response};
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
    ServerDown,
    ServerDown500,
    CaptchaFailedSolve,
    CaptchaUsed,
    CaptchaWrong,
    Registration,
    Nickname,
    Kicked,
    Unknown,
    Reqwest(reqwest::Error),
    Anyhow(anyhow::Error),
    Other,
}

impl From<reqwest::Error> for LoginErr {
    fn from(error: reqwest::Error) -> Self {
        LoginErr::Reqwest(error)
    }
}

impl From<anyhow::Error> for LoginErr {
    fn from(error: anyhow::Error) -> Self {
        LoginErr::Anyhow(error)
    }
}

impl Display for LoginErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            LoginErr::ServerDown => SERVER_DOWN_ERR,
            LoginErr::ServerDown500 => SERVER_DOWN_500_ERR,
            LoginErr::CaptchaFailedSolve => CAPTCHA_FAILED_SOLVE_ERR,
            LoginErr::CaptchaUsed => CAPTCHA_USED_ERR,
            LoginErr::CaptchaWrong => CAPTCHA_WG_ERR,
            LoginErr::Registration => REG_ERR,
            LoginErr::Nickname => NICKNAME_ERR,
            LoginErr::Kicked => KICKED_ERR,
            LoginErr::Unknown => UNKNOWN_ERR,
            LoginErr::Reqwest(e) => return write!(f, "{}", e),
            LoginErr::Anyhow(e) => return write!(f, "{}", e),
            LoginErr::Other => "Other error", // Menambahkan kasus untuk LoginErr::Other
        };
        write!(f, "{}", message)
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
    sxiv: bool,
    manual_captcha: bool,
) -> Result<String, LoginErr> {
    let login_url = format!("{}/{}", base_url, page_php);
    let resp = client.get(&login_url).send()?;
    
    if resp.status() == StatusCode::BAD_GATEWAY {
        return Err(LoginErr::ServerDown);
    }
    
    let resp_text = resp.text()?;
    let doc = Document::from(resp_text.as_str());

    let mut params = vec![
        ("action", "login".to_owned()),
        ("lang", LANG.to_owned()),
        ("nick", username.to_owned()),
        ("pass", password.to_owned()),
        ("colour", color.to_owned()),
    ];

    if let Some(captcha_node) = doc.find(And(Name("input"), Attr("name", "challenge"))).next() {
        let captcha_value = captcha_node.attr("value").context("Captcha value not found")?;
        let captcha_img = doc.find(Name("img")).next().context("Captcha image not found")?.attr("src").context("Captcha image source not found")?;

        let captcha_input = if manual_captcha {
            handle_manual_captcha(captcha_img, sxiv)?
        } else {
            captcha::solve_b64(captcha_img).ok_or(LoginErr::CaptchaFailedSolve)?
        };

        params.extend(vec![
            ("challenge", captcha_value.to_owned()),
            ("captcha", captcha_input),
        ]);
    }

    let mut resp = client.post(&login_url).form(&params).send()?;
    
    match resp.status() {
        StatusCode::BAD_GATEWAY => return Err(LoginErr::ServerDown),
        StatusCode::INTERNAL_SERVER_ERROR => return Err(LoginErr::ServerDown500),
        _ => {}
    }

    handle_refresh(client, base_url, &mut resp)?;

    let resp_text = resp.text()?;
    check_login_errors(&resp_text)?;

    let doc = Document::from(resp_text.as_str());
    let iframe = doc.find(Attr("name", "view")).next().context("View iframe not found")?;
    let iframe_src = iframe.attr("src").context("Iframe source not found")?;

    let session_captures = SESSION_RGX.captures(iframe_src).context("Session not found in iframe source")?;
    let session = session_captures.get(1).context("Session capture group not found")?.as_str();
    
    Ok(session.to_owned())
}

fn handle_manual_captcha(captcha_img: &str, sxiv: bool) -> Result<String, LoginErr> {
    let base64_str = captcha_img.strip_prefix("data:image/png;base64,")
        .or_else(|| captcha_img.strip_prefix("data:image/png;base64,"))
        .context("Unexpected captcha image format")?;

    let img_decoded = general_purpose::STANDARD.decode(base64_str).context("Failed to decode base64 image")?;
    let img = image::load_from_memory(&img_decoded).context("Failed to load image from memory")?;
    let img_buf = image::imageops::resize(
        &img,
        img.width() * 4,
        img.height() * 4,
        image::imageops::FilterType::Nearest,
    );
    img_buf.save("captcha.gif").context("Failed to save captcha image")?;

    let captcha_input = if sxiv {
        display_captcha_sxiv()?
    } else {
        display_captcha_termage(&img)?
    };

    Ok(captcha_input)
}

fn display_captcha_sxiv() -> Result<String, LoginErr> {
    let mut sxiv_process = Command::new("sxiv")
        .arg("captcha.gif")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to open image with sxiv")?;

    let mut captcha_input = String::new();
    print!("Please enter the CAPTCHA: ");
    io::stdout().flush().context("Failed to flush stdout")?;
    io::stdin().read_line(&mut captcha_input).context("Failed to read CAPTCHA input")?;
    trim_newline(&mut captcha_input);

    sxiv_process.kill().context("Failed to close sxiv")?;

    Ok(captcha_input)
}

fn display_captcha_termage(img: &DynamicImage) -> Result<String, LoginErr> {
    termage::display_image("captcha.gif", img.width(), img.height());

    let mut captcha_input = String::new();
    print!("captcha: ");
    io::stdout().flush().context("Failed to flush stdout")?;
    io::stdin().read_line(&mut captcha_input).context("Failed to read CAPTCHA input")?;
    trim_newline(&mut captcha_input);

    Ok(captcha_input)
}

fn handle_refresh(client: &Client, base_url: &str, resp: &mut Response) -> Result<(), LoginErr> {
    while let Some(refresh_header) = resp.headers().get("refresh").and_then(|v| v.to_str().ok()) {
        let rgx = Regex::new(r#"URL=(.+)"#).unwrap();
        let refresh_url = format!(
            "{}{}",
            base_url,
            rgx.captures(refresh_header)
                .context("Failed to capture refresh URL")?
                .get(1)
                .context("Failed to get refresh URL capture")?
                .as_str()
        );
        println!("Waitroom enabled, waiting 10 seconds");
        thread::sleep(Duration::from_secs(10));
        *resp = client.get(refresh_url).send()?;
    }
    Ok(())
}

fn check_login_errors(resp_text: &str) -> Result<(), LoginErr> {
    if resp_text.contains(CAPTCHA_USED_ERR) {
        Err(LoginErr::CaptchaUsed)
    } else if resp_text.contains(CAPTCHA_WG_ERR) {
        Err(LoginErr::CaptchaWrong)
    } else if resp_text.contains(REG_ERR) {
        Err(LoginErr::Registration)
    } else if resp_text.contains(NICKNAME_ERR) {
        Err(LoginErr::Nickname)
    } else if resp_text.contains(KICKED_ERR) {
        Err(LoginErr::Kicked)
    } else {
        Ok(())
    }
}

pub fn logout(
    client: &Client,
    base_url: &str,
    page_php: &str,
    session: &str,
) -> Result<()> {
    let full_url = format!("{}/{}", base_url, page_php);
    let params = [("action", "logout"), ("session", session), ("lang", LANG)];
    client.post(&full_url).form(&params).send()?;
    Ok(())
}
