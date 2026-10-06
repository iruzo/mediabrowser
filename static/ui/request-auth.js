async function request(url, options) {
  const response = await fetch(url, options);
  if (response.redirected && new URL(response.url).pathname === "/api/login") {
    location.assign("/api/login");
    throw new Error("Login required");
  }
  if (!response.ok) throw new Error(await response.text());
  return response;
}
