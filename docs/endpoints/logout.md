# GET /api/logout

Requires the optional `auth` feature; the `api` feature is not required.

It removes the current session from memory, expires its cookie, and returns
`200 OK` with an empty body. It never returns HTML or redirects, independently
of the `ui` feature. Other sessions remain valid. With `ui,auth`, choose `logout`
from the bundled interface's menu to call this endpoint and reload the UI.

Requests without a valid session also expire the cookie and return 200.
Other methods return 405. Responses use `Cache-Control: no-store`.

Using cookies saved by [login](./login.md):

```sh
curl -b cookies.txt -c cookies.txt http://localhost:30003/api/logout
```
