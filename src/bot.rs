use crate::config::CONFIG;
use crate::live::{StreamArgs, LIVES};
use crate::user::AUTH;
use once_cell::sync::{Lazy, OnceCell};
use std::collections::HashMap;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::sync::RwLock;

pub(crate) static BOT: OnceCell<Bot> = OnceCell::new();

static ROOMS: Lazy<RwLock<HashMap<i64, StreamArgs>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "可用命令如下：")]
enum UserCommand {
    #[command(description = "显示本帮助信息。")]
    Help,
    #[command(description = "认证Bot使用权限，看到这条消息的时候你已经用不到这条指令了。")]
    Start(String),
    #[command(description = "显示用户信息及使用情况。")]
    Status,

    #[command(description = "设置推流码。")]
    Key(String),
    #[command(description = "设置拉流参数。")]
    Args(String),
    #[command(description = "清空拉流参数。")]
    ClearArgs,
    #[command(description = "快速设置 Niconico 的拉流参数")]
    Niconico(String),
    #[command(description = "设置推流链接，并开始推流。")]
    Live(String),
    #[command(description = "结束推流。")]
    End,
}

async fn handle_user_command(
    bot: Bot,
    message: Message,
    command: UserCommand,
) -> anyhow::Result<()> {
    let chat_id = message.chat.id;
    if let UserCommand::Start(secret) = command {
        if secret == CONFIG.secret {
            AUTH.write().await.create(chat_id).await?;
            bot.send_message(chat_id, "Welcome!".to_string()).await?;
        }
        return Ok(());
    } else if !AUTH.read().await.has_permission(chat_id).await {
        anyhow::bail!("Unauthorized");
    }

    let result: anyhow::Result<()> = try {
        match command {
            UserCommand::Help => {
                bot.send_message(chat_id, UserCommand::descriptions().to_string())
                    .await?;
            }
            UserCommand::Status => {
                bot.send_message(chat_id, format!("用户 ID：{chat_id}"))
                    .await?;
            }
            UserCommand::Key(key) => {
                let room = AUTH.read().await.room(chat_id).await;
                let mut rooms = ROOMS.write().await;
                if let Some(room) = rooms.get_mut(&room.index()) {
                    room.live_key = key;
                } else {
                    rooms.insert(
                        room.index(),
                        StreamArgs {
                            room_index: room.index(),
                            args: Vec::new(),
                            live_key: key,
                            srt: false,
                        },
                    );
                }
                drop(rooms);
                bot.send_message(chat_id, "推流码已设置。").await?;
            }
            UserCommand::Args(_) | UserCommand::Niconico(_) => {
                let args = if let UserCommand::Args(args) = command {
                    shlex::split(&args).ok_or(anyhow::anyhow!("凭据解析失败"))?
                } else if let UserCommand::Niconico(user_session) = command {
                    vec!["--niconico-user-session".to_string(), user_session]
                } else {
                    unreachable!()
                };

                let room = AUTH.read().await.room(chat_id).await;
                let mut rooms = ROOMS.write().await;
                if let Some(room) = rooms.get_mut(&room.index()) {
                    room.args = args;
                    bot.send_message(chat_id, "凭据已设置。").await?;
                } else {
                    rooms.insert(
                        room.index(),
                        StreamArgs {
                            room_index: room.index(),
                            args,
                            live_key: "".to_string(),
                            srt: false,
                        },
                    );
                    bot.send_message(chat_id, "凭据已设置，待设置推流码。")
                        .await?;
                }
            }
            UserCommand::ClearArgs => {
                let room = AUTH.read().await.room(chat_id).await;
                let mut rooms = ROOMS.write().await;
                rooms.get_mut(&room.index()).map(|room| room.args.clear());
                bot.send_message(chat_id, "凭据已清空。").await?;
            }
            UserCommand::Live(url) => {
                let room = AUTH.read().await.room(chat_id).await;
                let room_id = room.index();
                let rooms = ROOMS.read().await;
                if let Some(room) = rooms.get(&room_id) {
                    let mut lives = LIVES.lock().await;
                    // shutdown previous live if exists
                    if let Some(mut live) = lives.remove(&room_id) {
                        if let Err(e) = tokio::try_join!(live.source.kill(), live.ffmpeg.kill()) {
                            bot.send_message(chat_id, format!("关闭直播失败：{}", e))
                                .await?;
                        }
                    }
                    // launch new live
                    let processes = room.build(&url, chat_id).await?;
                    lives.insert(room_id, processes);
                    drop(lives);

                    bot.send_message(chat_id, "推流已开始。").await?;
                } else {
                    bot.send_message(chat_id, "未配置推/拉流设置。").await?;
                }
            }
            UserCommand::End => {
                let room = AUTH.read().await.room(chat_id).await;
                if let Some(mut live) = LIVES.lock().await.remove(&room.index()) {
                    if let Err(e) = tokio::try_join!(live.source.kill(), live.ffmpeg.kill()) {
                        bot.send_message(chat_id, format!("关闭直播失败：{}", e))
                            .await?;
                    }
                } else {
                    bot.send_message(chat_id, "直播未开始。").await?;
                }
            }
            _ => {}
        }
    };

    if let Err(e) = result {
        bot.send_message(chat_id, e.to_string()).await?;
    }

    Ok(())
}

pub async fn start() -> anyhow::Result<()> {
    let bot = Bot::new(&CONFIG.token);
    let _ = BOT.set(bot.clone());
    let handler = dptree::entry().branch(
        Update::filter_message()
            .filter_command::<UserCommand>()
            .endpoint(handle_user_command),
    );
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}
