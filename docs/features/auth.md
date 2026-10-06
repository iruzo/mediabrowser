# Authentication

The optional `auth` Cargo feature requires login for file and protected account
operations. It is disabled by default and works independently of `api`, `ui`,
`httpd`, `https`, `cors`, and `firewall`.

## Enable

Run with the bundled UI:

```sh
cargo run --locked --features auth
```

Build an authenticated API backend without the bundled UI:

```sh
cargo build --locked --release --no-default-features --features api,auth
```

For containers with HTTPS:

```sh
FEATURES=api,ui,auth,https docker compose --profile pro up --build
```

Authentication endpoints require only `auth`; file API endpoints require `api`.
Their responses are identical with or without `ui`: success returns an empty
`200 OK`, and missing sessions return `401 Unauthorized`. Authentication APIs
never return HTML or redirects. See the endpoint references:

- [POST /api/login](../endpoints/login.md)
- [POST /api/password](../endpoints/password.md)
- [GET /api/logout](../endpoints/logout.md)

Without `auth`, these paths have no special routing and files have no account
access control. The former `/login`, `/password`, and `/logout` paths have no
special routing either.

## Bundled UI

With `ui,auth`, visit `/ui/`. The interface opens a login dialog when an API
request returns 401. Its menus include `change password` and `logout`.

- Login reloads the current UI path after success.
- Changing the password closes the dialog and keeps the current session.
- Logout reloads the UI and displays login again.
- The static UI shell is public, including deep UI paths. File data and account
  operations remain protected; unauthenticated UI requests do not resolve paths
  on disk.

`build.rs` selects `request-auth.js` instead of `request.js`, inserts
`static/ui/auth.html` at the `AUTH_UI` marker, appends `auth.css`, and includes
`auth.js` before the shared scripts. These additions require both `ui` and
`auth`; other builds omit them. They create no extra endpoints or asset requests.
The forms submit URL-encoded requests through the same APIs as custom frontends.

## Accounts and permissions

Create account directories inside `DATA_DIR` (`/data` by default):

```text
/data/
  .home/
    alice/
    bob/
  .root/
  shared/
```

- Usernames are case-sensitive directory names. Account directories must be
  real directories, not symbolic links.
- `root` exists only when `.root/` exists; `.home/root/` does not create it.
- There is no registration endpoint or registration UI.
- Set initial passwords before exposing accounts: anyone who reaches an account
  without `.shadow` can set its first password.
- To reset a password, remove the account's `.shadow` file and set its password
  again before exposing it. Restart the server to invalidate all sessions.

Regular users can read and modify shared files and their own `.home/<username>/`.
They cannot access `.root/`, another user's home, `.shadow`, or `.shadow.*` paths,
or create, replace, move, or remove account directories. Listings, searches,
recursive copies, and TAR downloads enforce the same restrictions.

Root bypasses account restrictions, including access to password files. It can
manage accounts through the file API. General path validation, symlink rules,
and protection of the mounted data root still apply.

## Password storage

- Passwords must be nonempty. They have no configured maximum length and are
  never truncated; spaces and Unicode are preserved.
- Form decoding streams passwords into SHA-512, then hashes the 64-byte result
  with Argon2id: 19 MiB memory, two iterations, one lane, a random 128-bit salt,
  and 512-bit output. Processing memory stays bounded regardless of length.
- `.shadow` stores 80 binary bytes: a 16-byte salt followed by a 64-byte hash.
  Algorithm parameters are fixed in code. New password files use mode `0600`
  on Unix. Hash size does not imply equivalent password entropy.
- The previous text format migrates after successful verification. Failed login
  leaves it untouched. Migration and password changes use `.shadow.new` followed
  by atomic replacement; an existing temporary file causes replacement to fail.
- Password changes invalidate the user's other sessions and retain the current
  one. Other users' sessions remain valid.

## Sessions

Sessions use random opaque cookies stored in process memory. Restarting the
server requires everyone to log in again. Cookies have `HttpOnly`,
`SameSite=Strict`, and `Path=/`; `https` adds `Secure`. Use HTTPS to protect
passwords and cookies in transit. There is no session timeout or persistent
cookie expiration.

Protected requests without a session return 401, including CORS preflights.
All responses with `auth` enabled use `Cache-Control: no-store`.

## Custom frontend and origin checks

An origin consists of scheme, hostname, and port; paths do not matter.
`http://localhost:30003/app/` and `http://localhost:30003/api/login` share an
origin. Changing the port, hostname, subdomain, or HTTP/HTTPS scheme changes it.

For browser POST requests, the server compares a supplied `Origin` with its
listener scheme and the request's `Host`. A mismatch returns 403. This helps
prevent another website from submitting actions using a user's session.
Direct API clients may omit `Origin`, but protected endpoints still require a
valid session cookie.

Serve a custom frontend and proxy `/api/` under one public origin. Call relative
URLs such as `/api/login`, submit URL-encoded credentials, and handle 200 and 401
responses in the frontend. The browser stores and sends the HttpOnly cookie;
JavaScript cannot read it.

The proxy must preserve the public `Host`, including its port. The backend
listener must match the public HTTP/HTTPS scheme. HTTPS at the proxy with plain
HTTP to the backend fails the origin check; the server does not interpret
`Forwarded` or `X-Forwarded-Proto`.

Cross-origin browser sessions are unsupported, even with `cors`: origin checks
still apply, cookies use `SameSite=Strict`, CORS omits
`Access-Control-Allow-Credentials`, and session checks precede preflight handling.

## Source files

- Endpoint handlers: `src/endpoints/api/login.rs`, `password.rs`, and `logout.rs`.
- Sessions and password storage: `src/core/auth.rs`.
- Credential parsing: `src/core/credentials.rs`.
- UI additions: `static/ui/auth.html`, `auth.css`, `auth.js`, and `request-auth.js`.
