use futures::stream::{self, StreamExt};
use std::collections::HashSet;

use axum::async_trait;
use serenity::all::Interaction;
use serenity::{
    all::{ChannelId, Message, Ready},
    client::{Context, EventHandler},
};

use tokio::sync::mpsc::Sender;
use tracing::{error, info};

pub enum DiscordMessage {
    Received(Message),
}

pub struct Handler {
    tx: Sender<DiscordMessage>,
    allowed_channels: HashSet<ChannelId>,
}

impl Handler {
    pub fn new(tx: Sender<DiscordMessage>, allowed_channels: HashSet<ChannelId>) -> Self {
        Self {
            tx,
            allowed_channels,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        dbg!(&interaction);

        match interaction {
            Interaction::Component(ref component) => {
                if component.data.custom_id.starts_with("recap-") {
                    crate::services::commands::recap::publish(&ctx, &interaction)
                        .await
                        .unwrap();
                    None
                } else {
                    println!("Received unknown component: {component:#?}");
                    None
                }
            }
            Interaction::Autocomplete(command) => match command.data.name.as_str() {
                "recap" => {
                    crate::services::commands::recap::autocomplete(&ctx, &command)
                        .await
                        .unwrap();
                    None
                }
                _ => None,
            },
            Interaction::Command(command) => match command.data.name.as_str() {
                "recap" => crate::services::commands::recap::run(&ctx, &command)
                    .await
                    .unwrap(),
                _ => None,
            },
            _ => {
                println!("Received unknown interaction: {interaction:#?}");
                None
            }
        };
    }

    async fn message(&self, _: Context, msg: Message) {
        info!("Message: {:?}", msg);
        if !self.allowed_channels.contains(&msg.channel_id) {
            return;
        }
        if let Err(e) = self.tx.send(DiscordMessage::Received(msg)).await {
            error!("Could not send received message tx over channel: {e}");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let http = &ctx.http;

        stream::iter(ready.guilds)
            .for_each(|guild| async move {
                println!("Connected to guild: {guild:#?}");
                let _commands = guild
                    .id
                    .set_commands(http, vec![crate::services::commands::recap::register()])
                    .await;
            })
            .await;
    }
}
