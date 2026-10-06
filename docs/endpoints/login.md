# GET/POST /api/login

Requires the optional `auth` feature; the `api` feature is not required.
See [authentication](../features/auth.md) for account provisioning, password
storage, cookies, and access rules.

GET displays a centered form with username, password, and login button.
The endpoint is public. Unauthenticated requests to protected endpoints
redirect here with `303 See Other`.

POST accepts `application/x-www-form-urlencoded` fields:

- `username`: the existing account's directory name, or `root` for `.root/`.
- `password`: a nonempty password, with no configured maximum length.

The first successful login creates `.shadow` if it is missing. Later logins
must match the stored password. Password setup is serialized; login never
replaces an existing password, apart from migrating the previous storage format
after verification. An invalid or linked `.shadow` prevents login. New password
files have mode `0600` on Unix. There is no registration option.

Successful login creates an in-memory session, sets its `HttpOnly` cookie, and
redirects with 303 to `/ui/` when UI is enabled, otherwise `/`. If the request
contains an existing session cookie, that session is replaced. Responses use
`Cache-Control: no-store`.

Invalid credentials return 401 with the login form. Malformed forms return 400,
unsupported content types 415, unsupported methods 405, and login failures
caused by storage or internal errors 500. An Origin header, when supplied,
must match the server's scheme and Host; otherwise POST returns 403.

For a command-line session:

```sh
curl -c cookies.txt \
  --data-urlencode 'username=alice' \
  --data-urlencode 'password=your-password' \
  http://localhost:30003/api/login
```

The server holds sessions only in memory; restarting it requires a new login.
