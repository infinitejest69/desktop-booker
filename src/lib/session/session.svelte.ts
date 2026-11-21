import { getContext, setContext } from "svelte";
import { api } from "../api";
import type { Permission } from "../bindings/Permission";
import type { SessionStatus } from "../bindings/SessionStatus";
import type { User } from "../bindings/User";
import { allows } from "./permissions";

export class Session {
  status = $state<SessionStatus | null>(null);
  readonly user = $derived(this.status?.user ?? null);

  async refresh(): Promise<void> {
    this.status = await api.sessionStatus();
  }

  can(permission: Permission): boolean {
    return allows(this.user?.role, permission);
  }

  signedIn(user: User): void {
    if (this.status) this.status = { ...this.status, user, needs_setup: false };
  }

  async lock(): Promise<void> {
    await api.logout().catch(() => {});
    if (this.status) this.status = { ...this.status, user: null };
    await this.refresh().catch(() => {});
  }
}

const KEY = Symbol("session");

export function provideSession(session: Session): void {
  setContext(KEY, session);
}

export function useSession(): Session {
  return getContext<Session>(KEY);
}
