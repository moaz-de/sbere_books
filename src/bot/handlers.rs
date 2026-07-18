use std::sync::Arc;

use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{InputFile,InlineKeyboardMarkup},
};
use uuid::Uuid;

use crate::{
    config::Config,
    database::{self as db, DbPool},
    models::{DialogueState, PendingContribution},
};
use super::keyboards::{
    self as kb, back_to_main_keyboard, BrowseItem,
    contribute_cancel_keyboard, finish_contribution_keyboard,
    main_menu_keyboard, search_results_keyboard,
};

pub type MyDialogue = Dialogue<DialogueState, InMemStorage<DialogueState>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// ============================================================
//  /start
// ============================================================

pub async fn handle_start(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(
        msg.chat.id,
        format!("👋 أهلا بكم في مكتبة {}", cfg.library_name),
    )
    .reply_markup(main_menu_keyboard())
    .await?;
    Ok(())
}

// ============================================================
//  القائمة الرئيسية (callback)
// ============================================================

pub async fn handle_main_menu(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
) -> HandlerResult {
    dialogue.exit().await?;
    let msg = q.message.as_ref().unwrap();
    bot.edit_message_text(
        msg.chat().id,
        msg.id(),
        format!("👋 أهلا بكم في مكتبة {}", cfg.library_name),
    )
    .reply_markup(main_menu_keyboard())
    .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

// ============================================================
//  تصفح المجلدات
// ============================================================

pub async fn handle_browse_root(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
) -> HandlerResult {
    show_folder_page(&bot, &q, &pool, None, 1, &cfg).await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

pub async fn handle_open_folder(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
) -> HandlerResult {
    let data = q.data.as_deref().unwrap_or("");
    if let Some(folder_id) = data.strip_prefix("open_folder:") {
        show_folder_page(&bot, &q, &pool, Some(folder_id), 1, &cfg).await?;
    }
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

pub async fn handle_folder_page(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
) -> HandlerResult {
    let data = q.data.as_deref().unwrap_or("");
    if let Some(rest) = data.strip_prefix("page:") {
        // "page:<folder_id>:<page_num>"
        if let Some(colon) = rest.rfind(':') {
            let folder_id = &rest[..colon];
            let page: usize = rest[colon + 1..].parse().unwrap_or(1);
            show_folder_page(&bot, &q, &pool, Some(folder_id), page, &cfg).await?;
        }
    } else if let Some(rest) = data.strip_prefix("page_root:") {
        let page: usize = rest.parse().unwrap_or(1);
        show_folder_page(&bot, &q, &pool, None, page, &cfg).await?;
    }
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

pub async fn handle_open_file(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<DbPool>,
) -> HandlerResult {
    let data = q.data.as_deref().unwrap_or("");
    let msg = q.message.as_ref().unwrap();

    if let Some(file_pk) = data.strip_prefix("open_file:") {
        match db::get_file(&pool, file_pk).await? {
            Some(file) => {
                bot.send_document(msg.chat().id, InputFile::file_id(&file.file_id))
                    .caption(&file.name)
                    .await?;
            }
            None => {
                bot.answer_callback_query(&q.id)
                    .text("❌ الملف غير موجود.")
                    .await?;
                return Ok(());
            }
        }
    }
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

async fn show_folder_page(
    bot: &Bot,
    q: &CallbackQuery,
    pool: &DbPool,
    folder_id: Option<&str>,
    page: usize,
    cfg: &Config,
) -> HandlerResult {
    let msg = q.message.as_ref().unwrap();

    let sub_folders = db::get_subfolders(pool, folder_id).await?;
    let files = match folder_id {
        Some(id) => db::get_files(pool, id).await?,
        None => vec![],
    };

    let items: Vec<BrowseItem> = sub_folders
        .into_iter()
        .map(BrowseItem::Folder)
        .chain(files.into_iter().map(BrowseItem::File))
        .collect();

    let title = match folder_id {
        Some(id) => match db::get_folder(pool, id).await? {
            Some(f) => format!("📂 {}", f.name),
            None => "📂 المجلد".to_string(),
        },
        None => format!("📚 مكتبة {}", cfg.library_name),
    };

    let parent_id: Option<String> = match folder_id {
        Some(id) => db::get_folder(pool, id).await?.and_then(|f| f.parent_id),
        None => None,
    };

    let keyboard = kb::folder_keyboard(
        &items,
        page,
        cfg.page_size,
        parent_id.as_deref(),
        folder_id,
    );

    bot.edit_message_text(msg.chat().id, msg.id(), title)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

// ============================================================
//  البحث
// ============================================================

pub async fn handle_search_start(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> HandlerResult {
    let msg = q.message.as_ref().unwrap();
    dialogue.update(DialogueState::AwaitingSearchKeyword).await?;
    bot.edit_message_text(msg.chat().id, msg.id(), "🔎 أدخل كلمة البحث:")
        .reply_markup(InlineKeyboardMarkup::new(vec![vec![
            teloxide::types::InlineKeyboardButton::callback("❌ إلغاء", "cancel_search"),
        ]]))
        .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

pub async fn handle_search_keyword(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
) -> HandlerResult {
    let keyword = msg.text().unwrap_or("").trim().to_lowercase();
    if keyword.is_empty() {
        bot.send_message(msg.chat.id, "⚠️ الرجاء إدخال كلمة بحث.").await?;
        return Ok(());
    }

    let results = db::search_files(&pool, &keyword).await?;
    dialogue.exit().await?;

    if results.is_empty() {
        bot.send_message(
            msg.chat.id,
            format!("❌ لا توجد نتائج تطابق \"{}\"", keyword),
        )
        .reply_markup(back_to_main_keyboard())
        .await?;
        return Ok(());
    }

    let keyboard = search_results_keyboard(&results, 1, cfg.page_size);
    bot.send_message(
        msg.chat.id,
        format!("🔍 نتائج البحث ({} نتيجة):", results.len()),
    )
    .reply_markup(keyboard)
    .await?;
    Ok(())
}

pub async fn handle_cancel_search(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
) -> HandlerResult {
    dialogue.exit().await?;
    let msg = q.message.as_ref().unwrap();
    bot.edit_message_text(
        msg.chat().id,
        msg.id(),
        format!("👋 أهلا بكم في مكتبة {}", cfg.library_name),
    )
    .reply_markup(main_menu_keyboard())
    .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

// ============================================================
//  نظام المساهمة
// ============================================================

pub async fn handle_contribute_start(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> HandlerResult {
    let msg = q.message.as_ref().unwrap();
    dialogue.update(DialogueState::AwaitingContributeInfo).await?;
    bot.edit_message_text(
        msg.chat().id,
        msg.id(),
        "📚 شكرًا لرغبتك في المساهمة بالمكتبة\n\n\
         🔹 الرجاء إدخال معلومات المنهج أو الكورس\n\
         مع ذكر القسم المناسب (مثال: علوم الحاسوب - لغة C++).",
    )
    .reply_markup(contribute_cancel_keyboard())
    .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

pub async fn handle_contribute_info(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
) -> HandlerResult {
    let text = msg.text().unwrap_or("").trim().to_string();
    if text.is_empty() {
        bot.send_message(msg.chat.id, "⚠️ الرجاء كتابة وصف للمواد.").await?;
        return Ok(());
    }
    dialogue
        .update(DialogueState::AwaitingContributeFile { description: text })
        .await?;
    bot.send_message(
        msg.chat.id,
        format!(
            "📂 الرجاء إرسال الملف/الملفات.\n\n\
             ✅ يمكنك إرسال أكثر من ملف.\n\
             ⚠️ الملفات المسموحة: PDF, DOCX, PPTX, ZIP, RAR, MP4\n\
             🚫 الحد الأقصى: {}MB\n\
             📌 إذا كان الملف أكبر تواصل مع: @{}",
            cfg.max_file_size_mb, cfg.admin_username
        ),
    )
    .reply_markup(finish_contribution_keyboard())
    .await?;
    Ok(())
}

// استبدل دالة handle_contribute_file السابقة بهذا التوقيع والمنطق المصحح:
pub async fn handle_contribute_file(
    bot: Bot,
    msg: Message,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
    dialogue: MyDialogue, // 💡 أضفنا الـ dialogue هنا لتصفير الحالة بعد النجاح
    description: String,
) -> HandlerResult {
    let (file_id, file_name, file_size) = extract_file_info(&msg);
    let file_id = match file_id {
        Some(id) => id,
        None => {
            bot.send_message(msg.chat.id, "❌ يرجى إرسال ملف صحيح.").await?;
            return Ok(());
        }
    };

    if let Some(size) = file_size {
        if size > cfg.max_file_size_bytes() {
            bot.send_message(
                msg.chat.id,
                format!(
                    "⚠️ الملف يتجاوز {}MB.\n📌 تواصل مع: @{}",
                    cfg.max_file_size_mb, cfg.admin_username
                ),
            )
            .await?;
            return Ok(());
        }
    }

    let allowed = [".pdf", ".docx", ".pptx", ".zip", ".rar", ".mp4"];
    if !allowed.iter().any(|e| file_name.to_lowercase().ends_with(e)) {
        bot.send_message(msg.chat.id, "❌ امتداد غير مدعوم. المسموح: PDF, DOCX, PPTX, ZIP, RAR, MP4").await?;
        return Ok(());
    }

    // إعادة توجيه الملف للقناة الخاصة للتخزين السحابي الفوري
    let forwarded = bot
        .forward_message(ChatId(cfg.storage_channel_id), msg.chat.id, msg.id)
        .await?;
    let (stored_file_id, _, _) = extract_file_info(&forwarded);
    let stored_file_id = stored_file_id.unwrap_or(file_id);

    let user_raw_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    let contribution = PendingContribution {
        id: Uuid::new_v4().to_string(),
        user_id: user_raw_id,
        username: msg.from().and_then(|u| u.username.clone()),
        description: description.clone(),
        file_id: stored_file_id,
        file_name: file_name.clone(),
    };
    db::insert_pending_contribution(&pool, &contribution).await?;

    // إنهاء حالة الحوار ليعود المستخدم إلى الوضع الافتراضي
    dialogue.exit().await?;

    let user_info = msg.from().map(|u| {
        format!("{} (@{})", u.first_name, u.username.as_deref().unwrap_or("بدون"))
    }).unwrap_or_else(|| "مجهول".to_string());

    bot.send_message(
        ChatId(cfg.admin_id),
        format!("📥 مساهمة من: {}\n📝 {}\n📎 {}", user_info, description, file_name),
    ).await?;

    bot.send_message(msg.chat.id, "🎉 شكرًا لك! تم استلام الملف وحفظه سحابياً بنجاح وسيقوم المشرف بمراجعته.")
        .reply_markup(back_to_main_keyboard())
        .await?;
    Ok(())
}



pub async fn handle_finish_contribution(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> HandlerResult {
    dialogue.exit().await?;
    let msg = q.message.as_ref().unwrap();
    bot.edit_message_text(
        msg.chat().id,
        msg.id(),
        "🎉 شكرًا على مساهمتك 🙏\n🌟 سيتم مراجعة الملفات قبل إضافتها.",
    )
    .reply_markup(back_to_main_keyboard())
    .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

// ============================================================
//  نبذة
// ============================================================

pub async fn handle_about(
    bot: Bot,
    q: CallbackQuery,
    cfg: Arc<Config>,
) -> HandlerResult {
    let msg = q.message.as_ref().unwrap();
    bot.edit_message_text(
        msg.chat().id,
        msg.id(),
        format!(
            "📖 *نبذة عن مكتبة {}*\n\n\
             مرحبًا بك في مكتبتنا التعليمية 🌟\n\n\
             1️⃣ توفير المناهج والكورسات لجميع الطلاب بسهولة.\n\
             2️⃣ العلم حق للجميع.\n\
             3️⃣ تعزيز التعاون بين الطلاب.\n\
             4️⃣ كسر الحواجز المالية.\n\
             5️⃣ تشجيع المساهمات الفردية.\n\n\
             🤝 شارك ملفاتك لدعم زملائك. معًا نحو بيئة تعليمية عادلة ✨",
            cfg.library_name
        ),
    )
    .parse_mode(teloxide::types::ParseMode::Markdown)
    .reply_markup(back_to_main_keyboard())
    .await?;
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}

// ============================================================
//  مساعد استخراج معلومات الملف
// ============================================================

fn extract_file_info(msg: &Message) -> (Option<String>, String, Option<u64>) {
    if let Some(doc) = msg.document() {
        return (
            Some(doc.file.id.clone()),
            doc.file_name.clone().unwrap_or_else(|| "file".to_string()),
            Some(doc.file.size.into()), // 💡 تحويل u32 إلى u64 لتفادي خطأ mismatched types
        );
    }
    if let Some(vid) = msg.video() {
        return (
            Some(vid.file.id.clone()),
            vid.file_name.clone().unwrap_or_else(|| "video.mp4".to_string()),
            Some(vid.file.size.into()), // 💡 تحويل u32 إلى u64 لتفادي خطأ mismatched types
        );
    }
    (None, "unknown".to_string(), None)
}



// 💡 ضع هذه الدالة في نهاية ملف handlers.rs تماماً لحل مشكلة صفحة البحث
pub async fn handle_search_page(
    bot: Bot,
    q: CallbackQuery,
    pool: Arc<DbPool>,
    cfg: Arc<Config>,
) -> HandlerResult {
    let data = q.data.as_deref().unwrap_or("");
    let msg = q.message.as_ref().unwrap();

    if let Some(page_str) = data.strip_prefix("search_page:") {
        let page: usize = page_str.parse().unwrap_or(1);
        
        let results = db::search_files(&pool, "").await?; 
        let keyboard = search_results_keyboard(&results, page, cfg.page_size);
        
        bot.edit_message_reply_markup(msg.chat().id, msg.id())
            .reply_markup(keyboard)
            .await?;
    }
    bot.answer_callback_query(&q.id).await?;
    Ok(())
}
