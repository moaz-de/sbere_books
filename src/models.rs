use serde::{Deserialize, Serialize};

// ============================================================
//  هياكل البيانات الرئيسية
// ============================================================

/// يمثّل مجلداً في شجرة المكتبة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// معرّف المجلد (UUID نصي)
    pub id: String,
    /// اسم المجلد
    pub name: String,
    /// معرّف المجلد الأب (None يعني أنه في الجذر)
    pub parent_id: Option<String>,
}

/// يمثّل ملفاً (كتاباً) مخزَّناً في التخزين السحابي لتليجرام
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// معرّف الملف (UUID نصي)
    pub id: String,
    /// اسم الملف أو الكتاب
    pub name: String,
    /// معرّف المجلد الذي يتبع له
    pub folder_id: String,
    /// الـ file_id الخاص بتليجرام — يُستخدم لإعادة إرسال الملف مباشرة
    pub file_id: String,
}

/// يمثّل مساهمة (ملف وصله البوت من مستخدم) قبل مراجعة الأدمن
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingContribution {
    pub id: String,
    /// معرّف مستخدم تليجرام الذي أرسل الملف
    pub user_id: i64,
    /// اسم المستخدم (اختياري)
    pub username: Option<String>,
    /// النص التوضيحي الذي أرسله المستخدم
    pub description: String,
    /// الـ file_id بعد الـ Forward للقناة الخاصة
    pub file_id: String,
    /// اسم الملف الأصلي
    pub file_name: String,
}

// ============================================================
//  حالات المحادثة (Dialogue States)
// ============================================================

/// حالات المستخدم داخل نظام الحوار (Dialogue)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum DialogueState {
    /// الحالة الافتراضية — لا توجد عملية جارية
    #[default]
    Idle,

    /// بانتظار النص التوضيحي للمساهمة
    /// (المستخدم كتب /contribute أو ضغط زر المساهمة)
    AwaitingContributeInfo,

    /// بانتظار الملف/الملفات بعد أن أدخل المستخدم النص التوضيحي
    AwaitingContributeFile {
        /// النص الذي أدخله المستخدم في الخطوة السابقة
        description: String,
    },

    /// بانتظار كلمة البحث من المستخدم
    AwaitingSearchKeyword,
}
