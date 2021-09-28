# teledustry

Manage your [Mindustry](https://github.com/Anuken/Mindustry) server through Telegram bot.

## Overview

teledustry can:
- Execute game server commands through Telegram bot commands
- Pipe the game server output to multiple Telegram chats
- Upload game map to server through Telegram bot

teledustry spawns the Mindustry game server as a child process and create pipes between game server's stdin & stdout and your Telegram bot.
teledustry reads game commands from the output of the `help` command of the game server, then combine them with teledustry's built-in commands (like `/output` or `/uploadmap`) to a command list uses as the bot's command list.

## Usage

1. Build teledustry by yourself or download the pre-build binary from [Releases](https://github.com/EAimTY/teledustry/releases)
2. Download Mindustry game server file from [here](https://anuke.itch.io/mindustry)
3. Talk to [@Botfather](https://t.me/botfather) to create your Telegram bot and get its API Token
4. Install [Java Runtime Environment(JRE)](https://developers.redhat.com/products/openjdk/download)
5. Run teledustry in the following way

```
Usage: teledustry [options] SERVER_FILE

Options:
    -t, --token TOKEN                (required) set Telegram Bot HTTP API token
    -u, --user TELEGRAM_USERNAME     (required)specify a Telegram user who can interact with this bot
    -p, --proxy PROXY                set proxy (supported: http, https, socks5)
    -w, --webhook-port WEBHOOK_PORT  set webhook port (1 ~ 65535) and run bot in webhook mode
    -h, --help                       print this help menu
```

For example, if your Minedustry game server file is located in the current directory of shell as `server.jar`, you can start teledustry by using:

```console
$ teledustry -t API_TOKEN -u YOUR_TELEGRAM_USERNAME server.jar
```

Now your Mindustry game server was started. Talk to your Telegram bot.

Please note: You have to use bot command `/output` to let your bot forwards game server log to current Telegram chat.

## Useful Bot Commands

- `/output` - Send the output to current Telegram chat
- `/stop_output` Stop sending the output to current Telegram chat
- `/help` - Print the help menu
- `/host [mapname] [mode]` - Open the server. Will default to survival and a random map if not specified
- `/pause <on/off>` - Pause or unpause the game
- `/stop` Stop hosting the server
- `/uploadmap` Upload a map to `config/maps/`
- `/reloadmaps` Reload all maps from disk

## Build

```bash
$ git clone https://github.com/EAimTY/teledustry.git && cd teledustry
$ cargo build --release
```
You can find the compiled binary in `target/release/`

## License

GNU General Public License v3.0
