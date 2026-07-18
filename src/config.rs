use anyhow::{Context, Result};
use std::env;

// ============================================================
//  وضع الاتصال
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionMode {
    /// اتصال مستمر — مناسب للتطوير المحلي
    Polling,
    /// استقبال تحديثات عبر Webhook — مناسب للإنتاج
    Webhook,
}

impl ConnectionMode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "webhook" => ConnectionMode::Webhook,
            _ => ConnectionMode::Polling,
        }
    }
}

// ============================================================
//  هيكل الإعدادات الرئيسي
// ============================================================

#[derive(Debug, Clone)]
pub struct Config {
    // ----- البوت -----
    /// توكن البوت من @BotFather
    pub bot_token: String,

    // ----- قاعدة البيانات -----
    pub bot_database_url: String,

    // ----- وضع الاتصال -----
    pub connection_mode: ConnectionMode,

    // ----- إعدادات Webhook -----
    /// الرابط الخارجي الكامل (مثال: https://example.com)
    pub webhook_url: Option<String>,
    /// المسار الذي يستقبل طلبات تليجرام (مثال: /webhook)
    pub webhook_path: String,
    /// منفذ الخادم المحلي
    pub port: u16,

    // ----- التخزين السحابي -----
    /// chat_id القناة الخاصة التي يُعاد توجيه الملفات إليها
    pub storage_channel_id: i64,

    // ----- الأدمن -----
    pub admin_id: i64,
    pub admin_username: String,

    // ----- واجهة المكتبة -----
    pub library_name: String,
    pub max_file_size_mb: u64,
    pub page_size: usize,
}

impl Config {
    /// يقرأ جميع الإعدادات من متغيرات البيئة
    /// استدعِ `dotenv::dotenv().ok()` قبل هذه الدالة في main
    pub fn from_env() -> Result<Self> {
        let bot_token = env::var("BOT_TOKEN")
            .context("BOT_TOKEN غير موجود في .env")?;

        // DATABASE_URL محجوز للـ Postgres من بيئة Replit، نستخدم BOT_DATABASE_URL للـ SQLite
        let database_url = env::var("BOT_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://library.db".to_string());

        let connection_mode = ConnectionMode::from_str(
            &env::var("CONNECTION_MODE").unwrap_or_else(|_| "polling".to_string()),
        );

        let webhook_url = env::var("WEBHOOK_URL").ok();

        let webhook_path = env::var("WEBHOOK_PATH")
            .unwrap_or_else(|_| "/webhook".to_string());

        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("PORT يجب أن يكون رقماً صحيحاً")?;

        let storage_channel_id: i64 = env::var("STORAGE_CHANNEL_ID")
            .context("STORAGE_CHANNEL_ID غير موجود في .env")?
            .parse()
            .context("STORAGE_CHANNEL_ID يجب أن يكون رقماً")?;

        let admin_id: i64 = env::var("ADMIN_ID")
            .context("ADMIN_ID غير موجود في .env")?
            .parse()
            .context("ADMIN_ID يجب أن يكون رقماً")?;

        let admin_username = env::var("ADMIN_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());

        let library_name = env::var("LIBRARY_NAME")
            .unwrap_or_else(|_| "SBERE_books".to_string());

        let max_file_size_mb: u64 = env::var("MAX_FILE_SIZE_MB")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .context("MAX_FILE_SIZE_MB يجب أن يكون رقماً")?;

        let page_size: usize = env::var("PAGE_SIZE")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .context("PAGE_SIZE يجب أن يكون رقماً")?;

        // التحقق: إذا كان الوضع Webhook يجب توفر WEBHOOK_URL
        if connection_mode == ConnectionMode::Webhook && webhook_url.is_none() {
            anyhow::bail!("CONNECTION_MODE=webhook يتطلب تعيين WEBHOOK_URL في .env");
        }

        Ok(Config {
            bot_token,
            bot_database_url: database_url,
            connection_mode,
            webhook_url,
            webhook_path,
            port,
            storage_channel_id,
            admin_id,
            admin_username,
            library_name,
            max_file_size_mb,
            page_size,
        })
    }

    /// الرابط الكامل للـ Webhook (URL + Path)
    pub fn webhook_full_url(&self) -> Option<String> {
        self.webhook_url.as_ref().map(|base| {
            format!(
                "{}{}",
                base.trim_end_matches('/'),
                self.webhook_path
            )
        })
    }

    /// الحد الأقصى لحجم الملف بالبايت
    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb * 1024 * 1024
    }
}
