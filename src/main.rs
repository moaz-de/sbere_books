mod bot;
mod config;
mod database;
mod models;

use std::sync::Arc;

use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};
use teloxide::utils::command::BotCommands; // 💡 أضف هذا السطر في أعلى main.rs لتفعيل ميزة الأكواد التلقائية للأوامر
use tracing::info;
use tracing_subscriber::EnvFilter;

use bot::{schema, Command};
use config::{Config, ConnectionMode};
use models::DialogueState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    info!("📚 تشغيل مكتبة {} ...", cfg.library_name);
    info!("🔗 وضع الاتصال: {:?}", cfg.connection_mode);

    let pool = database::init_db(&cfg.bot_database_url)?;
    info!("✅ قاعدة البيانات جاهزة");

    let bot = Bot::new(&cfg.bot_token);
    bot.set_my_commands(Command::bot_commands()).await?;

    let cfg_arc = Arc::new(cfg.clone());
    let pool_arc = Arc::new(pool);

    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![
            InMemStorage::<DialogueState>::new(),
            cfg_arc,
            pool_arc
        ])
        .enable_ctrlc_handler()
        .build();

    match cfg.connection_mode {
        ConnectionMode::Polling => {
            info!("🔄 Long Polling ...");
            dispatcher.dispatch().await;
        }
        ConnectionMode::Webhook => {
            let webhook_url = cfg
                .webhook_full_url()
                .expect("WEBHOOK_URL مطلوب في وضع Webhook");
            info!("🌐 Webhook: {}", webhook_url);

            bot.set_webhook(reqwest::Url::parse(&webhook_url)?).await?;

            let listener = teloxide::update_listeners::webhooks::axum(
                bot.clone(),
                teloxide::update_listeners::webhooks::Options::new(
                    std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port)),
                    reqwest::Url::parse(&webhook_url)?,
                ),
            )
            .await?;

            info!("✅ الخادم على المنفذ {}", cfg.port);
            dispatcher
                .dispatch_with_listener(listener, LoggingErrorHandler::new())
                .await;
        }
    }

    Ok(())
}
