# GET/POST /api/password

Requires the optional `auth` feature and an active session. The `api` feature
is not required. Unauthenticated requests redirect with 303 to `/api/login`.

Visit this endpoint directly in the browser. GET displays a centered form with
current password, new password, and change password button, in that order.

POST accepts `application/x-www-form-urlencoded` fields:

- `password`: the current password.
- `new_password`: the replacement password.

The account comes from the session, including for root; no username is accepted.
Both passwords are required and have no configured maximum length. Spaces and
Unicode are preserved.

A successful change atomically replaces `.shadow` with a fresh salt and 512-bit
hash, invalidates that user's other sessions, and keeps the current session.
Other users' sessions remain active. The response redirects with 303 to `/ui/`
when UI is enabled, otherwise `/`. See [authentication](../features/auth.md)
for password storage details.

A wrong current password returns 401 and leaves the password and sessions
unchanged. Invalid forms return 400, unsupported content types 415, unsupported
methods 405, and storage or internal failures 500. An Origin header, when
supplied, must match the server's scheme and Host; otherwise POST returns 403.
Responses use `Cache-Control: no-store`.

Using cookies saved by [login](./login.md):

```sh
curl -b cookies.txt \
  --data-urlencode 'password=your-password' \
  --data-urlencode 'new_password=your-new-password' \
  http://localhost:30003/api/password
```
