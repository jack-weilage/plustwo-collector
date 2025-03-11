use eyre::{Context, OptionExt, Result};
use twitch_api::{
    HelixClient,
    client::ClientDefault,
    eventsub::{
        Transport,
        channel::ChannelChatMessageV1,
        stream::{StreamOfflineV1, StreamOnlineV1},
    },
    twitch_oauth2::{TwitchToken, UserToken},
    types::UserId,
};

pub struct TwitchClient {
    pub client: HelixClient<'static, reqwest::Client>,
    pub token: UserToken,
}
impl TwitchClient {
    pub async fn new() -> Result<Self> {
        let client = HelixClient::with_client(
            reqwest::Client::default_client_with_name(Some("plustwo.live collector".parse()?))
                .wrap_err("constructing client with name")?,
        );

        Ok(Self {
            token: token_from_env(&client).await?,
            client,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn initialize_listeners(&mut self, session_id: &str) -> Result<()> {
        self.refresh_token().await?;

        let transport = Transport::websocket(session_id);
        let broadcaster_id = self
            .id_for_username(std::env::var("TWITCH_BROADCASTER_NAME")?.as_str())
            .await?
            .ok_or_eyre("No broadcaster found")?;
        let user_id = self
            .id_for_username(std::env::var("TWITCH_USER_NAME")?.as_str())
            .await?
            .ok_or_eyre("No user found")?;

        tracing::info!("retrieved broadcaster and user ids, subscribing to events");

        // Subscribe to stream online notifications
        self.client
            .create_eventsub_subscription(
                StreamOnlineV1::broadcaster_user_id(broadcaster_id.clone()),
                transport.clone(),
                &self.token,
            )
            .await?;

        // Subscribe to stream offline notifications
        self.client
            .create_eventsub_subscription(
                StreamOfflineV1::broadcaster_user_id(broadcaster_id.clone()),
                transport.clone(),
                &self.token,
            )
            .await?;

        // Subscribe to chat message notifications
        self.client
            .create_eventsub_subscription(
                ChannelChatMessageV1::new(broadcaster_id, user_id),
                transport,
                &self.token,
            )
            .await?;

        Ok(())
    }
    pub async fn refresh_token(&mut self) -> Result<()> {
        if self.token.is_elapsed() {
            tracing::warn!("token invalid, refreshing...");
            self.token.refresh_token(&self.client).await?;
        } else {
            tracing::info!("token still valid")
        }

        Ok(())
    }

    pub async fn id_for_username(&self, user: &str) -> Result<Option<UserId>> {
        let channel = self
            .client
            .get_channel_from_login(user, &self.token)
            .await
            .wrap_err("Failed to fetch channel from login")?;

        Ok(channel.map(|c| c.broadcaster_id))
    }
}

async fn token_from_env(client: &HelixClient<'static, reqwest::Client>) -> Result<UserToken> {
    tracing::info!("retrieving refresh token from env");

    UserToken::from_refresh_token(
        client,
        std::env::var("TWITCH_REFRESH_TOKEN")?.into(),
        std::env::var("TWITCH_CLIENT_ID")?.into(),
        Some(std::env::var("TWITCH_CLIENT_SECRET")?.into()),
    )
    .await
    .wrap_err("Failed to fetch user token")
}
