pub mod handlers;
pub mod keyboards;

use std::sync::Arc;

use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateFilterExt, UpdateHandler,
    },
    prelude::*,
    utils::command::BotCommands,
};

use crate::{
    config::Config,
    database::DbPool,
    models::DialogueState,
};
use handlers::*;

pub type DialogueStorage = InMemStorage<DialogueState>;

// ============================================================
//  الأوامر
// ============================================================

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "أوامر المكتبة:")]
pub enum Command {
    #[command(description = "بدء استخدام البوت")]
    Start,
}

// ============================================================
//  شجرة الـ Handlers
// ============================================================

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let cmd_handler = teloxide::filter_command::<Command, _>()
        .endpoint(handle_command);

    // إصلاح تمرير الحالات (Dialogue States) والتوافق مع دالة dptree::case!
    let message_handler = Update::filter_message()
        .enter_dialogue::<Message, DialogueStorage, DialogueState>()
        .branch(cmd_handler)
        .branch(
            dptree::case![DialogueState::AwaitingContributeInfo]
                .filter(|m: Message| m.text().is_some())
                .endpoint(handle_contribute_info),
        )
        .branch(
            dptree::case![DialogueState::AwaitingContributeFile { description }]
                .filter(|m: Message| m.document().is_some() || m.video().is_some())
                .endpoint(handle_contribute_file_state),
        )
        .branch(
            dptree::case![DialogueState::AwaitingSearchKeyword]
                .filter(|m: Message| m.text().is_some())
                .endpoint(handle_search_keyword),
        );

    let callback_handler = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, DialogueStorage, DialogueState>()
        .branch(cb("main_menu",         handle_main_menu))
        .branch(cb("browse_root",       handle_browse_root))
        .branch(cb("search_books",      handle_search_start))
        .branch(cb("cancel_search",     handle_cancel_search))
        .branch(cb("contribute",        handle_contribute_start))
        .branch(cb("finish_contribution", handle_finish_contribution))
        .branch(cb("about",             handle_about))
        .branch(cb_prefix("open_folder:", handle_open_folder))
        .branch(cb_prefix("open_file:",   handle_open_file))
        .branch(cb_prefix("page:",        handle_folder_page))
        .branch(cb_prefix("page_root:",   handle_folder_page))
        .branch(cb_prefix("search_page:", handle_search_page))
        .branch(
            dptree::filter(|q: CallbackQuery| q.data.as_deref() == Some("noop"))
                .endpoint(|bot: Bot, q: CallbackQuery| async move {
                    bot.answer_callback_query(&q.id).await?;
                    Ok(()) as HandlerResult
                }),
        );

    dialogue::enter::<Update, DialogueStorage, DialogueState, _>()
        .branch(message_handler)
        .branch(callback_handler)
}

// ── مساعدات بناء فروع الـ callback المصححة بالكامل ──────────────────────────

fn cb<H, Args>(trigger: &'static str, handler: H)
    -> dptree::Handler<'static, dptree::di::DependencyMap,
                       Result<(), Box<dyn std::error::Error + Send + Sync>>,
                       teloxide::dispatching::DpHandlerDescription> // 💡 قمنا بتغيير هذا النوع هنا
where
    H: dptree::di::Injectable<dptree::di::DependencyMap,
                               Result<(), Box<dyn std::error::Error + Send + Sync>>,
                               Args> // 💡 تم تصحيح نوع الـ Args ليطابق حاقن teloxide
     + Send + Sync + 'static,
    Args: Send + Sync + 'static,
{
    dptree::filter(move |q: CallbackQuery| q.data.as_deref() == Some(trigger))
        .endpoint(handler)
}

fn cb_prefix<H, Args>(prefix: &'static str, handler: H)
    -> dptree::Handler<'static, dptree::di::DependencyMap,
                       Result<(), Box<dyn std::error::Error + Send + Sync>>,
                       teloxide::dispatching::DpHandlerDescription> // 💡 قمنا بتغيير هذا النوع هنا أيضاً
where
    H: dptree::di::Injectable<dptree::di::DependencyMap,
                               Result<(), Box<dyn std::error::Error + Send + Sync>>,
                               Args>
     + Send + Sync + 'static,
    Args: Send + Sync + 'static,
{
    dptree::filter(move |q: CallbackQuery| {
        q.data.as_deref().map(|d| d.starts_with(prefix)).unwrap_or(false)
    })
    .endpoint(handler)
}

// ── معالج الأوامر ────────────────────────────────────────────

async fn handle_command(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
    cfg: Arc<Config>,
) -> HandlerResult {
    match cmd {
        Command::Start => handle_start(bot, dialogue, msg, cfg).await,
    }
}

// ── مرسِل ملف المساهمة (تعديل التوقيع ليتوافق مع dptree) ────

async fn handle_contribute_file_state(
    bot: Bot,
    msg: Message,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
    dialogue: MyDialogue,
    description: String, // مكتبة dptree تقوم بفك حقل الحماية وتمريره كـ String مباشرة هنا
) -> HandlerResult {
    handle_contribute_file(bot, msg, pool, cfg, dialogue, description).await
}

