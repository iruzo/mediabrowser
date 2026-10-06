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

Use the authentication endpoints:

- [GET/POST /api/login](../endpoints/login.md): log in or set the first password.
- [GET/POST /api/password](../endpoints/password.md): change your own password.
- [GET /api/logout](../endpoints/logout.md): end the current session.

These routes require only `auth`, even though their URLs are under `/api`.
With `auth` disabled, they have no special routing. The former `/login`,
`/password`, and `/logout` paths have no special routing either.

Create account directories and set their first passwords before exposing the
server to other people: anyone who can reach an account without `.shadow` can
set its initial password. For an administrator reset, remove its `.shadow` file
from the mounted volume and log in again before exposing the account. Restart
the server to invalidate all sessions.

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

Unauthenticated requests receive `303 See Other` to `/api/login`, including API and
CORS preflight requests. Authenticated responses and the login page use
`Cache-Control: no-store`. Browser POST requests must have a matching
Origin when supplied; direct API clients can omit Origin. Cross-origin browser
sessions are not supported, even with `cors` enabled. A reverse proxy must
preserve the public Host and scheme used by the server's HTTP/HTTPS listener.
