# GET /api/logout

Requires the optional `auth` feature; the `api` feature is not required.

Visit this endpoint directly in the browser. It removes the current session
from memory, expires its cookie, and redirects with `303 See Other` to
`/api/login`. Other sessions remain valid. There is no logout button in the UI.

Requests without a valid session also expire the cookie and redirect to login.
Other methods return 405. Responses use `Cache-Control: no-store`.

Using cookies saved by [login](./login.md):

```sh
curl -b cookies.txt -c cookies.txt http://localhost:30003/api/logout
```
