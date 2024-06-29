# fNordeingang status server
The status server for the fNordeingang hackerspace.
## operation
To set the space to opened just send a GET-Request to `\<IP-ADDRESS\>:13337/api/[open:open_intern:close]`. Internally, this state change will be routed to the action modules.
## configuration
To configure the server a config toml file can be passed via the `--config` parameter. The supplied file should have the following format.
```toml
# Telegram API
telegram_api_key = "..."
telegram_chat_id_public = ...
telegram_chat_id_private = ...

# State
last_state = 0
last_state_change = 0

# Messages
general_close = ""
general_open = ""
member_close = ""
member_open = ""

# Server
api_key = "..."
api_port = 13337
api_address = "[::]"
rate_limiter_tokens = 3
rate_limiter_timeout = 300

# Spaceapi
space_name = "fNordeingang"
logo = "https://fnordeingang.de/wp-content/uploads/2013/06/logo_final21.png"
url = "https://fnordeingang.de/"
address = "Körnerstr. 72, 41464 Neuss, Germany"
latitude = 51.186234
longitude = 6.692624
email = "verein@fnordeingang.de"
mastodon = "@fnordeingang@telefant.net"
issue_mail = "vorstand@fnordeingang.de"

```
## authorization
Append an `Api-Key` header to every request, which contains the api key specified while compiling.