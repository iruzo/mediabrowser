# CORS

The `cors` feature enables cross-origin browser access without adding
dependencies.

```sh
CORS_ORIGIN=http://localhost:8080 cargo run --locked --features cors
```

Set `CORS_ORIGIN` to one HTTP(S) origin, including its scheme, host, and
optional port, without a path or trailing slash. Use `*` to allow any origin.
An unset or empty value disables CORS; invalid values stop startup.

IPv6 origins use brackets:

```sh
CORS_ORIGIN='http://[::1]:8080' cargo run --locked --features cors
```

Responses include `Access-Control-Allow-Origin`, including errors and redirects.
CORS preflight requests receive `204 No Content` and allow `GET` and `POST`
with `Content-Type` and `Range` request headers. Credentialed requests are not
