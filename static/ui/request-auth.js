async function request(url, options) {
  const response = await fetch(url, options);
  if (response.status === 401) {
    showAuth("login");
    throw new Error("Login required");
  }
  if (!response.ok) throw new Error(await response.text());
  return response;
}
