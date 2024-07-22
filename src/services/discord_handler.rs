use futures::stream::{self, StreamExt};
use std::collections::HashSet;

use axum::async_trait;
use serenity::all::{
    Command, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId, Interaction,
};
// use serenity::framework::standard::macros::{command, group, help, hook};
// use serenity::model::id::UserId;
use serenity::{
    all::{ChannelId, Message, Ready},
    client::{Context, EventHandler},
};

// use serenity::framework::standard::{
//     help_commands, Args, BucketBuilder, CommandGroup, CommandResult, DispatchError, HelpOptions,
//     StandardFramework,
// };
// use serenity::utils::{content_safe, ContentSafeOptions};

use tokio::sync::mpsc::Sender;
use tracing::{error, info};

pub enum DiscordMessage {
    Received(Message),
}

// #[group]
// #[commands(say)]
// struct General;

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
        if let Interaction::Command(command) = interaction {
            println!("Received command interaction: {command:#?}");

            let content = match command.data.name.as_str() {
                "recap" => {
                    crate::services::commands::recap::run(&ctx, &command)
                        .await
                        .unwrap();
                    None
                }
                _ => Some("not implemented :(".to_string()),
            };

            if let Some(content) = content {
                let data = CreateInteractionResponseMessage::new().content(content);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    println!("Cannot respond to slash command: {why}");
                }
            }
        }
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

// async fn clear_global_commands(ctx: &Context) {
//     // Fetch all existing global commands
//     let commands = ctx.http.get_global_commands().await.unwrap();
//
//     // Iterate and delete each command
//     for command in commands {
//         ctx.http.delete_global_command(command.id).await.unwrap();
//     }
//
//     println!("All global commands cleared.");
// }

// #[command]
// async fn say(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     match args.single_quoted::<String>() {
//         Ok(x) => {
//             let settings = if let Some(guild_id) = msg.guild_id {
//                 // By default roles, users, and channel mentions are cleaned.
//                 ContentSafeOptions::default()
//                     // We do not want to clean channal mentions as they do not ping users.
//                     .clean_channel(false)
//                     // If it's a guild channel, we want mentioned users to be displayed as their
//                     // display name.
//                     .display_as_member_from(guild_id)
//             } else {
//                 ContentSafeOptions::default()
//                     .clean_channel(false)
//                     .clean_role(false)
//             };
//
//             let content = content_safe(&ctx.cache, x, &settings, &msg.mentions);
//
//             msg.channel_id.say(&ctx.http, &content).await?;
//
//             return Ok(());
//         }
//         Err(_) => {
//             msg.reply(ctx, "An argument is required to run this command.")
//                 .await?;
//             return Ok(());
//         }
//     };
// }
//
// #[hook]
// async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
//     match command_result {
//         Ok(()) => println!("Processed command '{command_name}'"),
//         Err(why) => println!("Command '{command_name}' returned error {why:?}"),
//     }
// }
//
// #[hook]
// async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
//     println!("Could not find command named '{unknown_command_name}'");
// }
//
// #[hook]
// async fn normal_message(_ctx: &Context, msg: &Message) {
//     println!("Message is not a command '{}'", msg.content);
// }
//
// #[hook]
// async fn delay_action(ctx: &Context, msg: &Message) {
//     // You may want to handle a Discord rate limit if this fails.
//     let _ = msg.react(ctx, '⏱').await;
// }
//
// #[hook]
// async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
//     if let DispatchError::Ratelimited(info) = error {
//         // We notify them only once.
//         if info.is_first_try {
//             let _ = msg
//                 .channel_id
//                 .say(
//                     &ctx.http,
//                     format!("Try this again in {} seconds.", info.as_secs()),
//                 )
//                 .await;
//         }
//     }
// }
//
// pub async fn make_framework() -> StandardFramework {
//     StandardFramework::new()
//         // Set a function to be called prior to each command execution. This provides the context
//         // of the command, the message that was received, and the full name of the command that
//         // will be called.
//         //
//         // Avoid using this to determine whether a specific command should be executed. Instead,
//         // prefer using the `#[check]` macro which gives you this functionality.
//         //
//         // **Note**: Async closures are unstable, you may use them in your application if you are
//         // fine using nightly Rust. If not, we need to provide the function identifiers to the
//         // hook-functions (before, after, normal, ...).
//         // .before(before)
//         // // Similar to `before`, except will be called directly _after_ command execution.
//         .after(after)
//         // Set a function that's called whenever an attempted command-call's command could not be
//         // found.
//         .unrecognised_command(unknown_command)
//         // Set a function that's called whenever a message is not a command.
//         .normal_message(normal_message)
//         // Set a function that's called whenever a command's execution didn't complete for one
//         // reason or another. For example, when a user has exceeded a rate-limit or a command can
//         // only be performed by the bot owner.
//         .on_dispatch_error(dispatch_error)
//         // Can't be used more than once per 5 seconds:
//         .bucket("emoji", BucketBuilder::default().delay(5))
//         .await
//         // Can't be used more than 2 times per 30 seconds, with a 5 second delay applying per
//         // channel. Optionally `await_ratelimits` will delay until the command can be executed
//         // instead of cancelling the command invocation.
//         // .bucket(
//         //     "complicated",
//         //     BucketBuilder::default()
//         //         .limit(2)
//         //         .time_span(30)
//         //         .delay(5)
//         //         // The target each bucket will apply to.
//         //         .limit_for(LimitedFor::Channel)
//         //         // The maximum amount of command invocations that can be delayed per target.
//         //         // Setting this to 0 (default) will never await/delay commands and cancel the invocation.
//         //         .await_ratelimits(1)
//         //         // A function to call when a rate limit leads to a delay.
//         //         .delay_action(delay_action),
//         // )
//         // .await
//         // The `#[group]` macro generates `static` instances of the options set for the group.
//         // They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
//         // #name is turned all uppercase
//         .help(&MY_HELP)
//         .group(&GENERAL_GROUP)
// }
//
// // The framework provides two built-in help commands for you to use. But you can also make your own
// // customized help command that forwards to the behaviour of either of them.
// #[help]
// // This replaces the information that a user can pass a command-name as argument to gain specific
// // information about it.
// #[individual_command_tip = "Hello! こんにちは！Hola! Bonjour! 您好! 안녕하세요~\n\n\
// If you want more information about a specific command, just pass the command as argument."]
// // Some arguments require a `{}` in order to replace it with contextual information.
// // In this case our `{}` refers to a command's name.
// #[command_not_found_text = "Could not find: `{}`."]
// // Define the maximum Levenshtein-distance between a searched command-name and commands. If the
// // distance is lower than or equal the set distance, it will be displayed as a suggestion.
// // Setting the distance to 0 will disable suggestions.
// #[max_levenshtein_distance(3)]
// // When you use sub-groups, Serenity will use the `indention_prefix` to indicate how deeply an item
// // is indented. The default value is "-", it will be changed to "+".
// #[indention_prefix = "+"]
// // On another note, you can set up the help-menu-filter-behaviour.
// // Here are all possible settings shown on all possible options.
// // First case is if a user lacks permissions for a command, we can hide the command.
// #[lacking_permissions = "Hide"]
// // If the user is nothing but lacking a certain role, we just display it.
// #[lacking_role = "Nothing"]
// // The last `enum`-variant is `Strike`, which ~~strikes~~ a command.
// #[wrong_channel = "Strike"]
// // Serenity will automatically analyse and generate a hint/tip explaining the possible cases of
// // ~~strikethrough-commands~~, but only if `strikethrough_commands_tip_in_{dm, guild}` aren't
// // specified. If you pass in a value, it will be displayed instead.
// async fn my_help(
//     context: &Context,
//     msg: &Message,
//     args: Args,
//     help_options: &'static HelpOptions,
//     groups: &[&'static CommandGroup],
//     owners: HashSet<UserId>,
// ) -> CommandResult {
//     let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
//     Ok(())
// }
