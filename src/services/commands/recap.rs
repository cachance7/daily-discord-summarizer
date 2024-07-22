use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use futures::stream::{self, StreamExt};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::info;

#[derive(Debug)]
struct SimpleMessage {
    content: String,
    username: String,
    timestamp: DateTime<Utc>,
}

async fn get_guild_id_from_channel(ctx: &Context, channel_id: ChannelId) -> Option<GuildId> {
    if let Ok(channel) = channel_id.to_channel(ctx).await {
        if let serenity::model::channel::Channel::Guild(channel) = channel {
            return Some(channel.guild_id);
        }
    }
    None
}

async fn get_recent_messages(
    ctx: &Context,
    channel_id: ChannelId,
) -> Result<Vec<SimpleMessage>, serenity::Error> {
    let now = Utc::now();
    let one_day_ago: DateTime<Utc> = now - Duration::days(7);
    let s_one_day_ago =
        serenity::model::Timestamp::from_unix_timestamp(one_day_ago.timestamp()).unwrap();

    info!("Getting messages from {} to {}", s_one_day_ago, now);

    let http = Arc::new(ctx.http.clone());
    let guild_id = get_guild_id_from_channel(ctx, channel_id).await.unwrap();

    let mut messages: Vec<SimpleMessage> = Vec::new();
    let mut last_message_id: Option<MessageId> = None;

    loop {
        let builder = match last_message_id {
            Some(val) => GetMessages::new().before(val).limit(100),
            None => GetMessages::new().limit(100),
        };
        info!("Fetching messages with {:?}", builder);

        let recent_messages = channel_id.messages(&http, builder).await?;

        if recent_messages.is_empty() {
            break;
        }

        info!("Got {} messages", recent_messages.len());

        let recent_messages_in_timeframe: Vec<Message> = recent_messages
            .iter()
            .filter(|msg| msg.timestamp >= s_one_day_ago)
            .cloned()
            .collect();

        info!(
            "{} messages in timeframe",
            recent_messages_in_timeframe.len()
        );

        let simple_messages: Vec<SimpleMessage> =
            stream::iter(recent_messages_in_timeframe.clone())
                .then({
                    let http = http.clone();
                    move |msg| {
                        let http = http.clone();
                        async move {
                            let user_id = msg.author.id;

                            let display_name = guild_id
                                .member(&http, user_id)
                                .await
                                .unwrap()
                                .display_name()
                                .to_string();

                            SimpleMessage {
                                content: msg.content.clone(),
                                username: display_name,
                                timestamp: Utc
                                    .timestamp_opt(msg.timestamp.unix_timestamp(), 0)
                                    .unwrap(),
                            }
                        }
                    }
                })
                .collect()
                .await;

        messages.extend(simple_messages);

        if recent_messages_in_timeframe.len() < recent_messages.len() {
            break;
        }

        last_message_id = recent_messages.last().map(|msg| msg.id);

        info!("Last message id: {:?}", last_message_id);
    }

    messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(messages)
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let channel_id = interaction.channel_id;
    // let guild_id = interaction.guild_id.unwrap();
    let messages = get_recent_messages(ctx, channel_id).await?;

    dbg!(messages);

    Ok(())
    // let modal = CreateQuickModal::new("About you")
    //     .timeout(std::time::Duration::from_secs(600))
    //     .short_field("First name")
    //     .short_field("Last name")
    //     .paragraph_field("Hobbies and interests");
    // let response = interaction.quick_modal(ctx, modal).await?.unwrap();
    //
    // let inputs = response.inputs;
    // let (first_name, last_name, hobbies) = (&inputs[0], &inputs[1], &inputs[2]);
    //
    // response
    //     .interaction
    //     .create_response(
    //         ctx,
    //         CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(
    //             format!("**Name**: {first_name} {last_name}\n\nHobbies and interests: {hobbies}"),
    //         )),
    //     )
    //     .await?;
    // Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("recap").description("Get a recap of old activity in the channel")
}
