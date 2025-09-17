use ::serenity::all::{EventHandler, GatewayIntents, Message};
use ::serenity::prelude::TypeMapKey;
use atrium_api::agent::atp_agent::store::MemorySessionStore;
use atrium_api::agent::atp_agent::AtpAgent;
use atrium_xrpc_client::reqwest::ReqwestClient;
use dotenvy::dotenv;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{OtlpExporterPipeline, WithExportConfig};
use opentelemetry_sdk::{trace as sdktrace, Resource};
use opentelemetry_semantic_conventions as semconv;
use poise::serenity_prelude as serenity;
use std::env;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::prelude::*;

mod fwd;

type AtpClient = AtpAgent<MemorySessionStore, ReqwestClient>;

struct Data {
    atp: AtpClient,
}

impl TypeMapKey for Data {
    type Value = Arc<Data>;
}
type Error = Box<dyn std::error::Error + Send + Sync>;
//type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

// Event handler
struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: serenity::prelude::Context, msg: Message) {
        println!("Received message in {:?}: {}", msg.channel_id, msg.content);
        // Check if this channel should be forwarded to Bluesky
        if fwd::should_forward_channel(&msg.channel_id.to_string()) {
            println!("Forwarding message to Bluesky...");
            let data = ctx.data.read().await;
            if let Some(bot_data) = data.get::<Data>() {
                println!("Found bot data, forwarding...");
                if let Err(e) = fwd::forward_message(&ctx, &msg, &bot_data.atp).await {
                    error!(
                        error = %e,
                        channel_id = %msg.channel_id,
                        author = %msg.author.name,
                        "Failed to forward message to SP chat"
                    );
                }
            }
        }
    }
}

async fn setup_atp_sess() -> anyhow::Result<AtpAgent<MemorySessionStore, ReqwestClient>> {
    let xrpc = ReqwestClient::new("https://bsky.social".to_string());
    let store = MemorySessionStore::default();
    let agent = AtpAgent::new(xrpc, store);

    let handle = std::env::var("ATP_HANDLE")?;
    let password = std::env::var("ATP_APP_PASSWORD")?;

    info!(handle = %handle, "Attempting ATP login");

    let res = agent.login(handle, password).await?;

    info!(handle = ?res.handle, "Successfully logged in to ATP");

    Ok(agent)
}

/// Initialize OpenTelemetry with OTLP exporter if configuration is present
fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Check if OpenTelemetry configuration is present
    let otel_enabled = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
        || env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_ok()
        || env::var("OTEL_SERVICE_NAME").is_ok();

    if !otel_enabled {
        info!("No OpenTelemetry configuration found, using basic tracing");
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
        return Ok(());
    }

    info!("🚀 Initializing OpenTelemetry (because we're fancy!)");

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new(semconv::resource::SERVICE_NAME, "discord-to-sp-bot"),
        KeyValue::new(
            semconv::resource::SERVICE_VERSION,
            env!("CARGO_PKG_VERSION"),
        ),
        KeyValue::new(semconv::resource::SERVICE_NAMESPACE, "discord-bridge"),
        KeyValue::new("deployment.environment", "production"), // 😎
        KeyValue::new("team.name", "chaos-engineering"),       // 🔥
    ]);

    // Set up OTLP exporter (defaults to http://localhost:4318)
    let otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());

    info!(endpoint = %otlp_endpoint, "Setting up OTLP exporter");

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(OtlpExporterPipeline.tonic().with_endpoint(&otlp_endpoint))
        .with_trace_config(
            sdktrace::config()
                .with_resource(resource)
                .with_sampler(sdktrace::Sampler::AlwaysOn), // Sample everything because YOLO
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Create OpenTelemetry layer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create subscriber with both console and OpenTelemetry layers
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            ),
        )
        .with(otel_layer)
        .init();

    info!("✨ OpenTelemetry initialized successfully!");
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize telemetry (with OpenTelemetry if configured)
    init_telemetry().expect("Failed to initialize telemetry");

    info!("🎉 Starting Discord to SP chat bridge bot!");

    let discord_token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    let atp = setup_atp_sess()
        .await
        .expect("Failed to set up ATP session");

    let user_data = Arc::new(Data { atp });

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let ud_clone = user_data.clone();
    let framework = poise::Framework::<Arc<Data>, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                //poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                // for all guilds we are in
                for guild in ctx.cache.guilds() {
                    poise::builtins::register_in_guild(ctx, &framework.options().commands, guild)
                        .await?;
                }
                Ok(ud_clone)
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("create client failed");

    {
        let mut data = client.data.write().await;
        data.insert::<Data>(user_data);
    }

    client.start().await.unwrap();
}
