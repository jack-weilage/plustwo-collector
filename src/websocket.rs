use eyre::{Context, Result};
use futures::StreamExt;
use tokio_tungstenite::tungstenite::{
    self, Error as TungsteniteError, error::ProtocolError as TungsteniteProtocolError,
    protocol::WebSocketConfig,
};
use twitch_api::{
    TWITCH_EVENTSUB_WEBSOCKET_URL,
    eventsub::{Event as TwitchEvent, EventsubWebsocketData, ReconnectPayload, WelcomePayload},
    types::Timestamp,
};

use crate::twitch::TwitchClient;

pub struct WebsocketClient {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}
impl WebsocketClient {
    pub async fn connect() -> Result<Self> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(64 << 20))
            .max_frame_size(Some(16 << 20))
            .accept_unmasked_frames(false);

        let (socket, _) = tokio_tungstenite::connect_async_with_config(
            TWITCH_EVENTSUB_WEBSOCKET_URL.as_str(),
            Some(config),
            false,
        )
        .await
        .wrap_err("Failed to connect to eventsub websocket")?;

        Ok(Self { socket })
    }

    pub async fn run(
        &mut self,
        client: &mut TwitchClient,
        mut f: impl AsyncFnMut(TwitchEvent, Timestamp) -> Result<()>,
    ) -> Result<()> {
        while let Some(msg) = self.socket.next().await {
            match msg {
                Ok(tungstenite::Message::Text(msg)) => {
                    let event = TwitchEvent::parse_websocket(&msg)?;
                    self.handle_event(client, event, &mut f).await?;
                }
                Ok(_) => continue,
                Err(TungsteniteError::Protocol(
                    TungsteniteProtocolError::ResetWithoutClosingHandshake,
                )) => {
                    println!("Connection reset, attempting reconnect...");

                    self.socket = Self::connect().await?.socket;
                    continue;
                }
                _ => todo!(),
            }
        }

        Ok(())
    }

    async fn handle_event(
        &self,
        client: &mut TwitchClient,
        event: EventsubWebsocketData<'_>,
        mut f: impl AsyncFnMut(TwitchEvent, Timestamp) -> Result<()>,
    ) -> Result<()> {
        match event {
            EventsubWebsocketData::Welcome {
                payload: WelcomePayload { session },
                ..
            }
            | EventsubWebsocketData::Reconnect {
                payload: ReconnectPayload { session },
                ..
            } => {
                client.initialize_listeners(&session.id).await?;

                Ok(())
            }

            EventsubWebsocketData::Notification { metadata, payload } => {
                f(payload, metadata.message_timestamp.into_owned()).await
            }
            EventsubWebsocketData::Keepalive { .. } => Ok(()),
            ev => todo!("{ev:#?}"),
        }
    }
}
