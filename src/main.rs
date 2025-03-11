use database::{DatabaseClient, EventKind, MessageKind};
use eyre::{Context, Result};
use twitch::TwitchClient;
use twitch_api::eventsub::{Event as TwitchEvent, Message, Payload};
use websocket::WebsocketClient;

mod database;
mod time;
mod twitch;
mod websocket;

#[tokio::main]
async fn main() -> Result<()> {
    // Initializing tracing
    tracing_subscriber::fmt::fmt()
        .with_writer(std::io::stderr)
        .init();

    let mut client = TwitchClient::new().await?;
    let db = DatabaseClient::new(&std::env::var("SUPABASE_PG_URL")?).await?;

    tracing::info!("twitch and database clients connected");

    loop {
        let mut ws = WebsocketClient::connect().await?;
        tracing::info!("websocket client connected");

        // TODO: more resilient handling of errors (log + restart)
        ws.run(&mut client, async |ev, timestamp| match ev {
            TwitchEvent::StreamOnlineV1(Payload {
                message: Message::Notification(payload),
                ..
            }) => {
                tracing::info!("stream online: {:#?}", payload);

                db.insert_event(
                    payload.started_at,
                    payload.broadcaster_user_id,
                    EventKind::BroadcastOnline,
                )
                .await?;

                Ok(())
            }
            TwitchEvent::StreamOfflineV1(Payload {
                message: Message::Notification(payload),
                ..
            }) => {
                tracing::info!("stream offline: {:#?}", payload);

                db.insert_event(
                    timestamp,
                    payload.broadcaster_user_id,
                    EventKind::BroadcastOffline,
                )
                .await?;

                Ok(())
            }
            TwitchEvent::ChannelChatMessageV1(Payload {
                message: Message::Notification(payload),
                ..
            }) => {
                let kind = match payload.message.text.as_str() {
                    s if s.starts_with("+2") || s.ends_with("+2") => MessageKind::PlusTwo,
                    s if s.starts_with("-2") || s.ends_with("-2") => MessageKind::MinusTwo,
                    _ => return Ok(()),
                };

                println!("Received {kind:?} from {}", payload.chatter_user_login);

                db.insert_message(
                    timestamp,
                    payload.chatter_user_id,
                    payload.broadcaster_user_id,
                    kind,
                )
                .await?;

                println!("added to database");

                Ok(())
            }
            _ => Ok(()),
        })
        .await?;
    }
}
