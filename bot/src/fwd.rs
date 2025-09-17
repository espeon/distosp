use crate::{AtpClient, Error};
use anyhow::anyhow;
use atrium_api::types::Unknown;
use poise::serenity_prelude::{Context, Message};
use std::collections::HashMap;
use tracing::{debug, info, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Forward a Discord message to its associated SP chat channel
#[instrument(
    skip(ctx, atp_client),
    fields(
        channel_id = %msg.channel_id,
        author = %msg.author.name,
        message_id = %msg.id,
        content_length = msg.content.len()
    )
)]
pub async fn forward_message(
    ctx: &Context,
    msg: &Message,
    atp_client: &AtpClient,
) -> Result<(), Error> {
    // Add some completely unnecessary but enterprise-grade telemetry attributes 🎭
    let current_span = Span::current();
    current_span.set_attribute(
        "discord.guild_id",
        msg.guild_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "dm".to_string()),
    );
    current_span.set_attribute("discord.channel_type", "text"); // Always text for now
    current_span.set_attribute("discord.has_attachments", msg.attachments.len() > 0);
    current_span.set_attribute("discord.attachment_count", msg.attachments.len() as i64);
    current_span.set_attribute("business.criticality", "mission_critical"); // Obviously 😎
    current_span.set_attribute("team.on_call", "chaos_engineering");

    // Skip messages from bots or that start with command prefix
    if msg.author.bot || msg.content.starts_with("~") {
        current_span.set_attribute("skip.reason", "bot_or_command");
        tracing::debug!(
            reason = "Bot message or command prefix detected",
            author_is_bot = msg.author.bot,
            "Skipping bot message or command"
        );
        return Ok(());
    }

    // Convert Discord message content to SP chat format
    let message_text = format_discord_message(ctx, msg).await?;

    // Skip empty messages (e.g., just attachments without text)
    if message_text.trim().is_empty() {
        current_span.set_attribute("skip.reason", "empty_content");
        tracing::debug!(
            reason = "Empty message content after formatting",
            "Skipping empty message"
        );
        return Ok(());
    }

    current_span.set_attribute("message.formatted_length", message_text.len() as i64);
    current_span.set_attribute(
        "message.compression_ratio",
        message_text.len() as f64 / msg.content.len().max(1) as f64,
    );

    // Get the streamer DID for this channel
    let streamer_did = get_streamer_for_channel(&msg.channel_id.to_string())?;
    current_span.set_attribute("sp.streamer_did", streamer_did.clone());
    current_span.set_attribute("sp.protocol", "atproto");
    current_span.set_attribute("sp.collection", "place.stream.chat.message");

    info!(streamer_did = %streamer_did, "Found streamer mapping for channel");

    // Create chat message using the lex types
    let session = atp_client.get_session().await.ok_or("No active session")?;
    current_span.set_attribute("atp.session_did", session.did.to_string());

    let chat_message = lex::place::stream::chat::message::RecordData {
        text: message_text.clone(),
        created_at: atrium_api::types::string::Datetime::now(),
        streamer: streamer_did.parse()?,
        facets: None,
        reply: None,
    };

    debug!(
        message_length = message_text.len(),
        "Created chat message record"
    );

    tracing::debug!(
        record_type = "place.stream.chat.message",
        record_text_length = message_text.len() as i64,
        "atp_record_created"
    );

    // Convert to Unknown using serde deserialization
    let record_unknown: Unknown = serde_json::from_value(serde_json::to_value(&chat_message)?)?;

    tracing::debug!(
        operation = "com.atproto.repo.createRecord",
        endpoint = "create_record",
        service_name = "atproto_api",
        "api_call_starting"
    );
    debug!("Making API call to create record");
    let result = atp_client
        .api
        .com
        .atproto
        .repo
        .create_record(
            atrium_api::com::atproto::repo::create_record::InputData {
                repo: session.did.clone().into(),
                collection: "place.stream.chat.message".parse()?,
                record: record_unknown,
                rkey: None,
                // do not validate as PDSes can't resolve lexicons yet
                validate: Some(false),
                swap_commit: None,
            }
            .into(),
        )
        .await?;

    // Record success metrics and attributes (because why not track EVERYTHING! 📈)
    current_span.set_attribute("atp.record_uri", result.uri.clone());
    current_span.set_attribute("atp.record_cid", format!("{:?}", result.cid));
    current_span.set_attribute("operation.success", true);
    current_span.set_attribute("sla.performance_tier", "premium"); // We're fancy! ✨

    tracing::debug!(
        destination = "sp_chat",
        record_uri = result.uri.clone(),
        latency_category = "sub_second",
        customer_impact = "positive",
        "message_forwarded_successfully"
    );

    info!(
        uri = %result.uri,
        cid = ?result.cid,
        "Successfully posted message to SP chat"
    );

    Ok(())
}

/// Format a Discord message for posting to SP chat
#[instrument(skip(ctx), fields(content_length = msg.content.len()))]
async fn format_discord_message(ctx: &Context, msg: &Message) -> anyhow::Result<String> {
    let mut content = msg.content.clone();

    if content.trim().is_empty() {
        return Err(anyhow!("no content found!"));
    }

    // Handle mentions - convert Discord mentions to readable format
    for user in &msg.mentions {
        let mention_pattern = format!("<@{}>", user.id);
        let display_name = user.display_name();
        content = content.replace(&mention_pattern, &format!("@{}", display_name));
    }

    // Handle channel mentions
    for channel_mention in &msg.mention_channels {
        let mention_pattern = format!("<#{}>", channel_mention.id);
        content = content.replace(&mention_pattern, &format!("#{}", channel_mention.name));
    }

    // Handle role mentions (convert to readable format)
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = ctx.cache.guild(guild_id) {
            for role in &msg.mention_roles {
                if let Some(role_obj) = guild.roles.get(role) {
                    let mention_pattern = format!("<@&{}>", role.get());
                    content = content.replace(&mention_pattern, &format!("@{}", role_obj.name));
                }
            }
        }
    }

    let author_info = format!("{} (Discord):", msg.author.display_name());

    // Format the final message
    let formatted = if content.trim().is_empty() {
        author_info
    } else {
        format!("{} {}", author_info, content.trim())
    };

    // Return the formatted message (SP chat may have different limits than Bluesky)
    Ok(formatted)
}

/// Get channel mappings from environment or configuration
/// Format: "discord_channel_id=streamer_did,another_id=another_did"
pub fn get_channel_mappings() -> HashMap<String, String> {
    let mut mappings = HashMap::new();

    if let Ok(mapping_str) = std::env::var("CHANNEL_MAPPINGS") {
        // Expected format: "discord_channel_id=streamer_did,another_id=another_did"
        // Using = as delimiter since DIDs contain colons (e.g., did:web:my.ball)
        for pair in mapping_str.split(',') {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() == 2 {
                mappings.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }
    } else {
        println!("No CHANNEL_MAPPINGS environment variable set");
    }

    mappings
}

/// Get the streamer DID for a given Discord channel
fn get_streamer_for_channel(channel_id: &str) -> Result<String, Error> {
    let mappings = get_channel_mappings();
    mappings
        .get(channel_id)
        .cloned()
        .ok_or_else(|| format!("No streamer mapping found for channel {}", channel_id).into())
}

/// Check if a channel should be forwarded to SP chat
pub fn should_forward_channel(channel_id: &str) -> bool {
    let mappings = get_channel_mappings();
    println!("Channel mappings: {:?}", mappings);
    mappings.contains_key(channel_id)
}
