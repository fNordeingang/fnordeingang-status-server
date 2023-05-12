# fNordeingang status server
The status server for the fNordeingang hackerspace.
## operation
To set the space to opened just send a GET-Request to \<IP-ADDRESS\>:1337/[open:close]. Internally this state change will be routed to the action modules.
## compilation
While compilation the following environment variables shall be passed to the compiler:
- TELEGRAM_API_TOKEN
- TELEGRAM_CHAT_ID
- SHARED_SECRET
## authorization
The SHARED_SECRET should be a random 256bit value, which must be known by client and server. For every request to the API an auth-token header is expected. To generate the token:
- Send a GET-Request to /auth_challenge
- Hmac\<Sha3_512\> the auth-challenge header with the shared secret as the salt 
