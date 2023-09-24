# fNordeingang status server
The status server for the fNordeingang hackerspace.
## operation
To set the space to opened just send a GET-Request to \<IP-ADDRESS\>:13337/api/[open:close]. Internally this state change will be routed to the action modules.
## configuration
To configure the server a config toml file can be passed via the `--config` parameter. The supplied file should have the following format.
```
api_key = "..."
telegram_api_key = "..."
telegram_chat_id = "..."

# optional
rate_limiter_timeout = "..."
rate_limiter_tokens = ",,,"
```
## authorization
Append an `Api-Key` header to every request, which contains the api key specified while compiling.