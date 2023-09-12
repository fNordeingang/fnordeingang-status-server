# fNordeingang status server
The status server for the fNordeingang hackerspace.
## operation
To set the space to opened just send a GET-Request to \<IP-ADDRESS\>:1337/[open:close]. Internally this state change will be routed to the action modules.
## compilation
While compilation the following environment variables shall be passed to the compiler:
- TELEGRAM_API_TOKEN
- TELEGRAM_CHAT_ID
- API_KEY
## authorization
Append an `Api-Key` header to every request, which contains the api key specified while compiling.