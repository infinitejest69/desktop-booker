// Before any scene: an admin called Andy (PIN 1234), signed in, and the demo
// hotel loaded, all through the bridge's real commands.
const bridge = "http://127.0.0.1:1430/invoke/";

async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch(bridge + cmd, { method: "POST", body: JSON.stringify(args) });
  const value = await response.json();
  if (!response.ok) throw new Error(`${cmd}: ${JSON.stringify(value)}`);
  return value as T;
}

interface Status {
  needs_setup: boolean;
  user: { id: string } | null;
  users: { id: string; name: string }[];
}

export default async function globalSetup(): Promise<void> {
  const status = await call<Status>("session_status");
  if (status.needs_setup) await call("create_first_admin", { name: "Andy", pin: "1234" });
  else if (!status.user) {
    const andy = status.users.find((u) => u.name === "Andy") ?? status.users[0];
    await call("login", { userId: andy.id, pin: "1234" });
  }
  await call("seed_demo_data");
  // A member of staff too, for the Users screen.
  const users = (await call<Status>("session_status")).users;
  if (!users.some((u) => u.name === "Sam"))
    await call("save_user", { user: { id: crypto.randomUUID(), name: "Sam", role: "staff", active: true }, pin: "5678" });
}
