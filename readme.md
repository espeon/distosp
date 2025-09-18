# distosp

A one-way Discord to Streamplace bridge.

## Requirements
- Rust
- ATProto account
- Discord bot token

## Setup Instructions

### Prerequisites
Before starting, make sure you have:
- **Rust installed** - Visit [rustup.rs](https://rustup.rs/) to install Rust and Cargo
- **Discord bot token** - See detailed steps below
- **ATProto account** - See detailed steps below

### Getting Your Discord Bot Token

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" section in the left sidebar
4. Click "Add Bot" if you haven't already
5. Under the "Token" section, click "Copy" to get your bot token
6. **Important**: Keep this token secret and never share it publicly
7. Under "Privileged Gateway Intents", enable any intents your bot needs
   - For this bot, you may need "Message Content Intent" if you want to read message contents
8. To add the bot to your server:
   - Go to the "OAuth2" > "URL Generator" section
   - Select "bot" scope and the permissions you need
   - Copy the generated URL and visit it to add the bot to your server

### Getting Your ATProto/Bluesky Credentials

1. **If you have a Bluesky account**:
   - Your identifier is your handle (e.g., `yourname.bsky.social`)
   - For password, you'll need to create an "App Password":
     - Go to Settings > Privacy and Security > App Passwords
     - Click "Add App Password" and give it a name
     - Copy the generated password (you won't see it again)

2. **If you're using a different ATProto server**:
   - Contact your PDS administrator for the correct login details
   - Use your account credentials as provided

### Step-by-Step Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd discordtosp
   ```

2. **Set up environment variables**
   ```bash
   # Copy the example environment file
   cp .env.example .env

   # Edit the .env file with your favorite text editor
   nano .env
   # or
   code .env
   ```

   Fill in these required fields in your `.env` file:
   - `DISCORD_TOKEN` - Your Discord bot token
   - `ATPROTO_*` - Your ATProto/Bluesky credentials
   - `CHANNEL_MAPPINGS` - Discord channel to ATProto streamer mappings (see below)
   - Any other required configuration values

3. **Build and run the application**
   ```bash
   # For development (with debug info)
   cargo run

   # For production (optimized build)
   cargo run --release
   ```

### Installation (Optional)

To install the binary to your system PATH so you can run it from anywhere:

```bash
cargo install --path .
```

After installation, you can run the program with just:
```bash
distosp
```

## Channel Mappings Configuration

The `CHANNEL_MAPPINGS` environment variable configures which Discord channels forward messages to specific ATProto streamers' chat channels.

### Format
```
CHANNEL_MAPPINGS=discord_channel_id=atp_did,discord_channel_id=atp_did,...
```

### Components

**Discord Channel ID**:
- The unique numeric ID of a Discord channel (e.g., `123456789012345678`)
- Get this by right-clicking on a channel in Discord and selecting "Copy Channel ID" (requires Developer Mode enabled)

**ATProto DID (Decentralized Identifier)**:
- The unique identifier of the streamer on the ATProto network
- Use an ATProto explorer such as [atp.tools](https://atp.tools) or [pdsls.dev](https://pdsls.dev) to find your account's DID
- DIDs look like `did:plc:abc123xyzblabla` or `did:web:example.com`

### Examples

```bash
# Single mapping
CHANNEL_MAPPINGS=123456789012345678=did:web:streamer1.example.com

# Multiple mappings
CHANNEL_MAPPINGS=123456789012345678=did:web:streamer1.example.com,987654321098765432=did:web:streamer2.example.com
```

### How It Works

1. **Message Filtering**: The bot only forwards messages from Discord channels that are explicitly mapped
2. **Formatting**: Discord messages are reformatted as: `Username (Discord): message content`
3. **Forwarding**: Messages get sent to the corresponding streamer's chat on the ATProto network
4. **ATProto Record**: Each message becomes a `place.stream.chat.message` record

### Getting the Required Information

**Discord Channel ID**:
1. Enable Developer Mode in Discord (Settings > App Settings > Advanced > Developer Mode)
2. Right-click the channel you want to map
3. Select "Copy Channel ID"

**ATProto DID**:
1. Visit an ATProto explorer like [atp.tools](https://atp.tools) or [pdsls.dev](https://pdsls.dev)
2. Search for the streamer's handle/username
3. Copy their DID (starts with `did:`)

### Troubleshooting

- **"cargo: command not found"** - Make sure Rust is properly installed and in your PATH
- **Build errors** - Try `cargo clean` then `cargo build` again
- **Missing environment variables** - Double-check your `.env` file has all required fields filled in
- **Discord bot issues** - Ensure your bot has the necessary permissions in your Discord server
