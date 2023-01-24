use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use chrono_tz::Europe::Copenhagen;
use chrono_tz::Tz;
use rocket::Shutdown;
use rocket::{get, routes};
use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth, Token};
use std::{env, fs};
use tokio::sync::OnceCell;
use tracing::info;

static NINE: u32 = 21;
static TEN: u32 = 22;
static ELEVEN: u32 = 23;

static CLIENT: OnceCell<AuthCodeSpotify> = OnceCell::const_new();

async fn spotify() -> AuthCodeSpotify {
    let oauth = OAuth {
        scopes: scopes!(
            "user-read-currently-playing",
            "app-remote-control",
            "user-modify-playback-state",
            "user-read-playback-state"
        ),
        redirect_uri: "http://127.0.0.1:8000/callback".to_owned(),
        ..Default::default()
    };

    let creds = Credentials::from_env().unwrap();
    let spotify = AuthCodeSpotify::with_config(creds, oauth, Config::default());

    if let Ok(token_json) = env::var("TOKEN") {
        let token: Token = serde_json::from_str(&token_json).unwrap();
        *spotify.token.lock().await.unwrap() = Some(token.clone());
        if token.is_expired() {
            spotify.refresh_token().await.unwrap();
        }
    } else if env::var("CI").is_ok() {
        panic!("Cannot obtain a token in CI");
    }

    spotify
}

#[get("/callback?<code>")]
async fn callback(shutdown: Shutdown, code: String) -> &'static str {
    let spotify = CLIENT.get_or_init(spotify).await;

    spotify.request_token(&code).await.ok();

    shutdown.notify();

    "Close me :)"
}

#[rocket::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let now: DateTime<Tz> = Utc::now().with_timezone(&Copenhagen);
    let weekday = now.weekday();

    let is_weekend = weekday == Weekday::Fri || weekday == Weekday::Sat;
    if is_weekend {
        return Ok(());
    }

    let spotify = CLIENT.get_or_init(spotify).await;

    if spotify.get_token().lock().await.unwrap().is_none() {
        let url = spotify.get_authorize_url(true).unwrap();
        info!("Had no token. To get a new one, visit {}", url);

        rocket::build()
            .mount("/", routes![callback])
            .launch()
            .await
            .ok();
    }

    let token = (*spotify.get_token().lock().await.unwrap())
        .clone()
        .unwrap();
    let token_json = serde_json::to_string(&token)?;
    info!("TOKEN='{}'", token_json);

    if env::var("CI").is_ok() {
        fs::write(
            env::var("GITHUB_OUTPUT").unwrap(),
            format!("TOKEN='{token_json}'"),
        )?;
    }

    let after_nine = now.clone().with_hour(NINE).unwrap() <= now;
    let after_ten = now.clone().with_hour(TEN).unwrap() <= now;
    let after_eleven = now.clone().with_hour(ELEVEN).unwrap() <= now;

    let devices = spotify.device().await?;

    let device = devices.first().unwrap().clone();

    let device_id: Option<&str> = device.id.as_deref();

    match (after_eleven, after_ten, after_nine) {
        (true, _, _) => {
            spotify.volume(50, device_id).await.ok();
        }
        (false, true, _) => {
            spotify.volume(75, device_id).await.ok();
        }
        (false, false, true) => {
            spotify.volume(90, device_id).await.ok();
        }
        (_, _, _) => (),
    };

    Ok(())
}
