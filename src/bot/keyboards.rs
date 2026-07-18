use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::models::{File, Folder};

// ============================================================
//  القائمة الرئيسية
// ============================================================

pub fn main_menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📚 المكتبة", "browse_root")],
        vec![InlineKeyboardButton::callback("🔍 بحث", "search_books")],
        vec![InlineKeyboardButton::callback("🙌 المساهمة", "contribute")],
        vec![InlineKeyboardButton::callback("ℹ️ نبذة", "about")],
    ])
}

// ============================================================
//  لوحة تصفح المجلد مع Pagination
// ============================================================

/// بيانات عنصر واحد في القائمة (مجلد أو ملف)
#[derive(Debug, Clone)]
pub enum BrowseItem {
    Folder(Folder),
    File(File),
}

impl BrowseItem {
    pub fn label(&self) -> String {
        match self {
            BrowseItem::Folder(f) => format!("📁 {}", f.name),
            BrowseItem::File(f) => format!("📄 {}", f.name),
        }
    }

    /// الـ callback_data المرتبطة بهذا العنصر
    pub fn callback(&self) -> String {
        match self {
            BrowseItem::Folder(f) => format!("open_folder:{}", f.id),
            BrowseItem::File(f) => format!("open_file:{}", f.id),
        }
    }
}

/// يبني لوحة أزرار Inline لتصفح مجلد مع Pagination
///
/// # المعاملات
/// - `items`       — قائمة العناصر (مجلدات + ملفات)
/// - `page`        — رقم الصفحة الحالية (يبدأ من 1)
/// - `page_size`   — عدد العناصر في الصفحة
/// - `parent_id`   — معرّف المجلد الأب لزر الرجوع (None = نحن في الجذر)
/// - `folder_id`   — معرّف المجلد الحالي (لبناء callback رجوع)
pub fn folder_keyboard(
    items: &[BrowseItem],
    page: usize,
    page_size: usize,
    parent_id: Option<&str>,
    folder_id: Option<&str>,
) -> InlineKeyboardMarkup {
    let total_pages = (items.len() + page_size - 1).max(1) / page_size.max(1);
    let start = (page.saturating_sub(1)) * page_size;
    let current_items = items.get(start..start + page_size).unwrap_or(&items[start..]);

    let mut rows: Vec<Vec<InlineKeyboardButton>> = current_items
        .iter()
        .map(|item| vec![InlineKeyboardButton::callback(item.label(), item.callback())])
        .collect();

    // ── أزرار التنقل بين الصفحات ──
    let mut nav: Vec<InlineKeyboardButton> = Vec::new();
    if page > 1 {
        let prev_cb = match folder_id {
            Some(id) => format!("page:{}:{}", id, page - 1),
            None => format!("page_root:{}", page - 1),
        };
        nav.push(InlineKeyboardButton::callback("⬅️", prev_cb));
    }
    if total_pages > 1 {
        nav.push(InlineKeyboardButton::callback(
            format!("{}/{}", page, total_pages),
            "noop",
        ));
    }
    if page < total_pages {
        let next_cb = match folder_id {
            Some(id) => format!("page:{}:{}", id, page + 1),
            None => format!("page_root:{}", page + 1),
        };
        nav.push(InlineKeyboardButton::callback("➡️", next_cb));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }

    // ── أزرار التحكم السفلية ──
    match parent_id {
        Some(pid) => {
            rows.push(vec![InlineKeyboardButton::callback(
                "🔙 رجوع",
                format!("open_folder:{}", pid),
            )]);
        }
        None => {
            // نحن في الجذر — عرض أزرار القائمة الرئيسية
            rows.push(vec![
                InlineKeyboardButton::callback("🏠 الرئيسية", "main_menu"),
                InlineKeyboardButton::callback("🙌 المساهمة", "contribute"),
                InlineKeyboardButton::callback("ℹ️ نبذة", "about"),
            ]);
        }
    }

    InlineKeyboardMarkup::new(rows)
}

// ============================================================
//  لوحة نتائج البحث مع Pagination
// ============================================================

pub fn search_results_keyboard(
    files: &[File],
    page: usize,
    page_size: usize,
) -> InlineKeyboardMarkup {
    let total_pages = ((files.len() + page_size - 1) / page_size).max(1);
    let start = (page.saturating_sub(1)) * page_size;
    let current = files.get(start..start + page_size).unwrap_or(&files[start..]);

    let mut rows: Vec<Vec<InlineKeyboardButton>> = current
        .iter()
        .map(|f| {
            vec![InlineKeyboardButton::callback(
                format!("📄 {}", f.name),
                format!("open_file:{}", f.id),
            )]
        })
        .collect();

    // ── تنقل الصفحات ──
    let mut nav: Vec<InlineKeyboardButton> = Vec::new();
    if page > 1 {
        nav.push(InlineKeyboardButton::callback(
            "⬅️",
            format!("search_page:{}", page - 1),
        ));
    }
    if total_pages > 1 {
        nav.push(InlineKeyboardButton::callback(
            format!("{}/{}", page, total_pages),
            "noop",
        ));
    }
    if page < total_pages {
        nav.push(InlineKeyboardButton::callback(
            "➡️",
            format!("search_page:{}", page + 1),
        ));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }

    rows.push(vec![InlineKeyboardButton::callback(
        "❌ إلغاء البحث",
        "cancel_search",
    )]);

    InlineKeyboardMarkup::new(rows)
}

// ============================================================
//  لوحة المساهمة
// ============================================================

pub fn contribute_cancel_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "🏠 الرئيسية",
        "main_menu",
    )]])
}

pub fn finish_contribution_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "📩 إنهاء المساهمة",
            "finish_contribution",
        )],
        vec![InlineKeyboardButton::callback("🏠 الرئيسية", "main_menu")],
    ])
}

// ============================================================
//  مساعد عام
// ============================================================

pub fn back_to_main_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "🏠 العودة إلى الرئيسية",
        "main_menu",
    )]])
}
