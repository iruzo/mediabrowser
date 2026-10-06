# Authentication

The optional `auth` Cargo feature requires login for all enabled endpoints.
It works independently of `api`, `ui`, `httpd`, `https`, `cors`, and `firewall`.
It is disabled by default.

```sh
cargo run --locked --features auth
# Containers, with HTTPS:
FEATURES=api,ui,auth,https docker compose --profile pro up --build
```

Accounts are provisioned by creating directories inside `DATA_DIR` (the mounted
`/data` volume by default). There is no registration endpoint or registration UI.

```text
/data/
  .home/
    alice/
    bob/
  .root/
  shared/
```

`alice` and `bob` are usernames. The `root` account exists only when `.root/`
exists; `.home/root/` does not create it. Account directories must be real
directories, not symbolic links. Usernames are case-sensitive.

Visit `/login`. The page contains username and password fields and a login
button. The first successful login to an existing account creates its `.shadow`
file using that password. Later logins must match it. Password setup is serialized,
and existing passwords are never replaced by login. An invalid or linked
`.shadow` file prevents login. On Unix, new password files have mode `0600`.

Create account directories and set their first passwords before exposing the
server to other people: anyone who can reach an account without `.shadow` can
set its initial password. For an administrator reset, remove its `.shadow` file
from the mounted volume and log in again before exposing the account. Restart
the server to invalidate all sessions.

Visit `/password` while logged in to change your own password, including for root.
GET displays current-password and new-password fields and a change button.
POST accepts `application/x-www-form-urlencoded` fields `password` (current)
and `new_password`. The account comes from the session; no username is accepted.
Both passwords are required and have no configured length limit. A wrong current
password returns 401 and leaves the password and sessions unchanged. Invalid
forms return 400, unsupported content types 415, and other methods 405.
A successful change atomically replaces `.shadow` with a fresh salt and 512-bit
hash, invalidates that user's other sessions, and keeps the current session.
It redirects with 303 to `/ui/` when enabled, otherwise `/`. Other users' sessions
remain active. No navigation button is added; visit the endpoint directly.
Unauthenticated requests redirect to `/login`; browser POSTs require a matching
Origin when supplied, as with other authenticated endpoints.

Regular users can read and modify shared files and their own `.home/<username>/`
contents. They cannot access `.root/`, another user's home, or any `.shadow` or
`.shadow.*` path. They cannot create, replace, move, or remove account directories.
Searches, listings, recursive copies, and TAR downloads apply these same rules.

Root bypasses all account access restrictions. It can access every file and
account directory inside `DATA_DIR`, including its own `.root/.shadow`, other
users' `.shadow` files, and `.shadow.*` files. Root's listings, copies, and
downloads include these files. Root can provision, move, or remove account
directories and manage password files through the existing file endpoints.
The server's general path validation, symlink policy, and endpoint behavior still
apply, including the existing protection of the mounted data root itself.

Passwords have no configured maximum length and are never truncated. The login
form is decoded incrementally and fed into SHA-512, then the 64-byte result is
hashed with Argon2id (19 MiB memory, two iterations, one lane, random 128-bit salt,
512-bit output), following [OWASP's Argon2id guidance](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html#argon2id).
This keeps password processing memory bounded independently of
password length. `.shadow` contains exactly 80 binary bytes: the 16-byte random
salt followed by the 64-byte hash. Algorithm names and parameters are fixed in
code and are not stored in the file. Salt is retained because it is required to
verify the password securely. The previous text format is converted atomically
after a successful login; a wrong password leaves it untouched. The temporary
`.shadow.new` file used during conversion and password changes is accessible only
to root through endpoints. An existing temporary file causes replacement to fail
without overwriting it or the current password.
Empty passwords are rejected. Spaces and Unicode
are preserved. Hash output size does not imply equivalent password entropy.

Sessions use random opaque cookies and are stored only in process memory.
Restarting the app requires everyone to log in again. Cookies use `HttpOnly`,
`SameSite=Strict`, and `Path=/`, with `Secure` when the `https` feature is enabled.
Use HTTPS to protect passwords and cookies in transit. The browser cookie has no
persistent expiration. There is no session timeout.

Visit `/logout` directly in the browser to end the current session. It removes
that session from memory, expires its cookie, and redirects to `/login`. Other
sessions remain valid. This endpoint accepts GET and has no UI button. With
`auth` disabled, `/login`, `/logout`, and `/password` have no special routing.

Unauthenticated requests receive `303 See Other` to `/login`, including API and
CORS preflight requests. Login itself is public. Successful login redirects to
`/ui/` when UI is enabled, otherwise `/`. Authenticated responses and the login
page use `Cache-Control: no-store`. Browser POST requests must have a matching
Origin when supplied; direct API clients can omit Origin. Cross-origin browser
sessions are not supported, even with `cors` enabled. A reverse proxy must
preserve the public Host and scheme used by the server's HTTP/HTTPS listener.
