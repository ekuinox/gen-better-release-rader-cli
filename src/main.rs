use anyhow::{bail, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
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
    let len = artists.len();
    for artist in artists {
        println!(" - {}", artist.name);
    }
    println!("Length: {}", len);

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
