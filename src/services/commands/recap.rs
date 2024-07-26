use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_english::{parse_date_string, parse_duration, Dialect, Interval};
use futures::stream::{self, StreamExt};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{error, info};

use crate::config::AppConfig;
use crate::gpt::SummaryConfig;

#[derive(Debug)]
struct SimpleMessage {
    content: String,
    username: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug)]
enum Timeframe {
    LastDay,
    LastWeek,
    LastMonth,
    Custom(DateTime<Utc>),
}
impl Timeframe {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "last_day" => Some(Timeframe::LastDay),
            "last_week" => Some(Timeframe::LastWeek),
            "last_month" => Some(Timeframe::LastMonth),
            date_or_duration => {
                let now = Utc::now();
                if let Ok(duration) = parse_duration(date_or_duration) {
                    if let Interval::Seconds(seconds) = duration {
                        let custom_time = now - Duration::seconds(seconds as i64);
                        Some(Timeframe::Custom(custom_time))
                    } else {
                        None
                    }
                } else if let Ok(date) = parse_date_string(date_or_duration, now, Dialect::Us) {
                    Some(Timeframe::Custom(date))
                } else {
                    None
                }
            }
        }
    }
}

async fn get_guild_id_from_channel(ctx: &Context, channel_id: ChannelId) -> Option<GuildId> {
    if let Ok(channel) = channel_id.to_channel(ctx).await {
        if let serenity::model::channel::Channel::Guild(channel) = channel {
            return Some(channel.guild_id);
        }
    }
    None
}

async fn process_message(
    msg: Message,
    members_by_id: Arc<HashMap<UserId, Member>>,
) -> SimpleMessage {
    let user_id = msg.author.id;

    let display_name = members_by_id
        .get(&user_id)
        .map(|member| member.display_name().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    SimpleMessage {
        content: msg.content.clone(),
        username: display_name.clone(),
        timestamp: Utc
            .timestamp_opt(msg.timestamp.unix_timestamp(), 0)
            .unwrap(),
    }
}

async fn get_recent_messages(
    ctx: &Context,
    channel_id: ChannelId,
    since: DateTime<Utc>,
) -> Result<Vec<SimpleMessage>, serenity::Error> {
    let now = Utc::now();

    info!("Getting messages from {} to {}", since, now);

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
            .filter(|msg| msg.timestamp.unix_timestamp() >= since.timestamp())
            .cloned()
            .collect();

        info!(
            "{} messages in timeframe",
            recent_messages_in_timeframe.len()
        );

        let http = ctx.http.clone();
        let members = guild_id.members(&http, None, None).await?;
        let members_by_id: Arc<std::collections::HashMap<UserId, Member>> = Arc::new(
            members
                .iter()
                .map(|member| (member.user.id, member.clone()))
                .collect(),
        );

        let simple_messages: Vec<SimpleMessage> =
            stream::iter(recent_messages_in_timeframe.clone())
                .then({
                    let members_by_id = members_by_id.clone();
                    move |msg| {
                        let members_by_id = members_by_id.clone();
                        process_message(msg, members_by_id)
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

pub async fn run(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<Option<String>, serenity::Error> {
    let channel_id = interaction.channel_id;
    let options = interaction.data.options();

    // Find option named 'since'
    let since = options.iter().find(|opt| opt.name == "since");

    info!("Got since: {:?}", since);

    let since_val = {
        if let Some(since_opt) = since {
            match since_opt.value {
                ResolvedValue::String(val) => Some(val),
                _ => None,
            }
        } else {
            None
        }
    };

    let value = since_val.unwrap_or("last_week");

    info!("Got since auto value: {:?}", value);
    let content = {
        if let Some(timeframe) = Timeframe::from_str(value) {
            info!("Got timeframe: {:?}", timeframe);
            let messages = match timeframe {
                Timeframe::LastDay => {
                    info!("Getting messages from last day");
                    let data = CreateInteractionResponseMessage::new()
                        .content(
                            "Processing recap for the past day, this might take a few seconds...",
                        )
                        .ephemeral(true);
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(why) = interaction.create_response(&ctx.http, builder).await {
                        println!("Cannot respond to slash command: {why}");
                    }

                    let now = Utc::now();
                    let one_day_ago: DateTime<Utc> = now - Duration::days(1);
                    get_recent_messages(ctx, channel_id, one_day_ago).await?
                }
                Timeframe::LastWeek => {
                    info!("Getting messages from last week");
                    let data = CreateInteractionResponseMessage::new()
                        .content(
                            "Processing recap for the past week, this might take a few seconds...",
                        )
                        .ephemeral(true);
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(why) = interaction.create_response(&ctx.http, builder).await {
                        println!("Cannot respond to slash command: {why}");
                    }

                    let now = Utc::now();
                    let one_week_ago: DateTime<Utc> = now - Duration::weeks(1);
                    get_recent_messages(ctx, channel_id, one_week_ago).await?
                }
                Timeframe::LastMonth => {
                    info!("Getting messages from last month");
                    let data = CreateInteractionResponseMessage::new()
                        .content(
                            "Processing recap for the past month, this might take a few seconds...",
                        )
                        .ephemeral(true);
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(why) = interaction.create_response(&ctx.http, builder).await {
                        println!("Cannot respond to slash command: {why}");
                    }

                    let now = Utc::now();
                    let one_month_ago: DateTime<Utc> = now - Duration::weeks(4);
                    get_recent_messages(ctx, channel_id, one_month_ago).await?
                }
                Timeframe::Custom(date) => get_recent_messages(ctx, channel_id, date).await?,
            };

            let config = AppConfig::load().unwrap();
            let formatted_messages: Vec<String> = messages
                .iter()
                .map(|msg| {
                    format!(
                        "{}: {}: {}",
                        msg.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        msg.username,
                        msg.content
                    )
                })
                .collect();
            let file_contents = formatted_messages.join("\n");

            match crate::gpt::summarize(
                &file_contents,
                SummaryConfig {
                    max_tokens: config.summary.max_tokens,
                    model: config.summary.model.to_string(),
                    prompt: config.summary.prompt.to_string(),
                    ..SummaryConfig::default()
                },
            )
            .await
            {
                Ok(txt) => Some(txt),
                Err(e) => {
                    error!("Could not summarize message log: {e}");
                    None
                }
            }
        } else {
            info!("Invalid since value");
            None
        }
    };

    if let Some(content) = content {
        let mut btn_id = "recap-".to_owned();

        let mut message_content = content.clone();
        message_content.push_str("\n\n*(Note: AI is dumb. If this message uses the wrong pronouns or produces bad vibes let @casey know and he will __correct__ the bot)*");
        btn_id.push_str(interaction.data.id.to_string().as_str());
        let button = CreateButton::new(btn_id).label("Publish");
        let components = CreateActionRow::Buttons(vec![button]);
        let data = CreateInteractionResponseFollowup::new()
            .content(message_content)
            .components(vec![components])
            .ephemeral(true);

        if let Err(why) = interaction.create_followup(&ctx.http, data).await {
            println!("Cannot respond to slash command: {why}");
            Ok(None)
        } else {
            Ok(Some("Command processed".to_string()))
        }
    } else {
        Ok(Some("Command not processed".to_string()))
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("recap")
        .description("Get a recap of old activity in the channel")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "since",
                "The date or relative time to start the recap from",
            )
            .set_autocomplete(true),
        )
}

pub async fn publish(ctx: &Context, interaction: &Interaction) -> Result<(), serenity::Error> {
    if let Some(component) = interaction.clone().message_component() {
        let content = component.message.content.clone();
        let message = CreateInteractionResponseMessage::new().content(content);
        let data = CreateInteractionResponse::Message(message);
        match component.create_response(&ctx.http, data).await {
            Ok(message) => Ok(message),
            Err(e) => {
                eprintln!("Failed to create follow-up: {:?}", e);
                Err(e.into())
            }
        }
    } else {
        eprintln!("Interaction is not a message component");
        Err(serenity::Error::Other("Interaction is not a message component").into())
    }
}

pub async fn autocomplete(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let choices = vec![
        AutocompleteChoice::new("Yesterday", "last_day"),
        AutocompleteChoice::new("Last week", "last_week"),
        AutocompleteChoice::new("Last month", "last_month"),
    ];

    let res = CreateAutocompleteResponse::new().set_choices(choices);
    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Autocomplete(res))
        .await
}
