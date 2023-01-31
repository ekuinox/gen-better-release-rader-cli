use anyhow::{bail, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{AlbumType, Market},
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
    let results = futures::future::join_all(artists.into_iter().map(|artist| {
        client.artist_albums_manual(
            artist.id,
            Some(AlbumType::Single), // ここVec使えないんか～～～い
            Some(Market::FromToken),
            Some(50),
            None,
        )
    }))
    .await;

    if results.iter().any(|r| r.is_err()) {
        println!("results include any error");
    }
    let albums = results
        .into_iter()
        .flatten()
        .map(|r| r.items)
        .flatten()
        .filter(|album| {
            if let Some(release_data) = &album.release_date {
                release_data.starts_with("2023-01")
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
