use anyhow::{bail, Result};
use chrono::{Duration, NaiveDate, Utc};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{AlbumType, FullArtist, Market, SimplifiedAlbum},
    scopes, AuthCodePkceSpotify, Config, Credentials, OAuth,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = get_client().await?;
    let mut artists = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let resp = if let Some(after) = after {
            client
                .current_user_followed_artists(Some(&after), Some(50))
                .await?
        } else {
            client.current_user_followed_artists(None, Some(50)).await?
        };
        artists.extend(resp.items);
        let Some(next) = resp.cursors.and_then(|cursor| cursor.after) else {
            break;
        };
        after = Some(next);
    }
    // let len = artists.len();
    // for artist in artists {
    //     println!(" - {}", artist.name);
    // }
    // println!("Length: {}", len);
    println!("start get albums");
    let albums = get_albums(&client, &artists).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("start get singles");
    let singles = get_singles(&client, &artists).await;
    // let comilation = get_compilation(&client, &artists).await;
    // let appears_on = get_appears_on(&client, &artists).await;

    let all_albums = vec![albums, singles].into_iter().flatten().collect::<Vec<_>>();

    let today = Utc::now().date_naive();
    
    // 先週の最終日
    let prev_week = today - Duration::weeks(1);

    let albums = all_albums
        .into_iter()
        .filter(|album| {
            if let Some(Ok(release_date)) = album
                .release_date
                .as_ref()
                .map(|s| NaiveDate::parse_from_str(&s, "%F"))
            {
                release_date > prev_week
            } else {
                false
            }
        })
        .collect::<Vec<_>>();

    for album in albums {
        let artists = album
            .artists
            .into_iter()
            .map(|artist| artist.name)
            .collect::<Vec<_>>()
            .join(", ");
        let (_, url) = album.external_urls.into_iter().next().unwrap_or_default();
        println!(" - {} by {} {}", album.name, artists, url);
    }

    Ok(())
}

async fn get_client() -> Result<impl BaseClient + OAuthClient> {
    let Some(creds) = Credentials::from_env() else { bail!("Credentials::from_env failed.") };

    let scopes = scopes!(
        "user-follow-read",
        "playlist-read-private",
        "playlist-modify-private",
        "playlist-modify-public"
    );
    let Some(oauth) = OAuth::from_env(scopes) else { bail!("OAuth::from_env failed.") };
    let config = Config {
        token_refreshing: true,
        token_cached: true,
        ..Default::default()
    };

    let mut spotify = AuthCodePkceSpotify::with_config(creds, oauth, config);

    let url = spotify.get_authorize_url(None)?;
    spotify.prompt_for_token(&url).await?;

    spotify.write_token_cache().await?;

    Ok(spotify)
}

macro_rules! albums_getter {
    ($name:ident, $album_type:expr) => {
        async fn $name(client: &impl OAuthClient, artists: &[FullArtist]) -> Vec<SimplifiedAlbum> {
            let results = futures::future::join_all(artists.into_iter().map(|artist| {
                client.artist_albums_manual(
                    artist.id.clone(),
                    Some($album_type),
                    Some(Market::FromToken),
                    Some(50),
                    None,
                )
            }))
            .await;


            if let Some(e) = results.iter().find_map(|r| r.as_ref().err()) {
                eprintln!("{e}");
            }
        
            results
                .into_iter()
                .flatten()
                .flat_map(|page| page.items)
                .collect()
        }
    };
}

albums_getter!(get_albums, AlbumType::Album);
albums_getter!(get_singles, AlbumType::Single);
albums_getter!(get_compilation, AlbumType::Compilation);
albums_getter!(get_appears_on, AlbumType::AppearsOn);
