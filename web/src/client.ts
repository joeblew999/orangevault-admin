import { ConnectError, createClient } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";
import { AdminService } from "../gen/orangevault_admin/v1/admin_pb.js";

export const TOKEN_KEY = "orangevault-admin.token";

let currentToken: string | null = null;

export function setAdminToken(token: string | null) {
  currentToken = token;
  if (token) {
    localStorage.setItem(TOKEN_KEY, token);
  } else {
    localStorage.removeItem(TOKEN_KEY);
  }
}

export function loadAdminToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function errorMessage(e: unknown, fallback: string): string {
  return e instanceof ConnectError ? e.message : fallback;
}

const transport = createConnectTransport({
  baseUrl: typeof window !== "undefined" ? window.location.origin : "/",
  fetch: ((input, init) => {
    const headers = new Headers(init?.headers);
    if (!headers.has("Authorization") && currentToken) {
      headers.set("Authorization", `Bearer ${currentToken}`);
    }
    return globalThis.fetch(input as RequestInfo, { ...init, headers });
  }) as typeof globalThis.fetch,
});

export const adminClient = createClient(AdminService, transport);
