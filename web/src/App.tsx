import { useCallback, useEffect, useState } from "react";
import {
  adminClient,
  errorMessage,
  loadAdminToken,
  setAdminToken,
} from "./client";
import type {
  Membership,
  Organization,
  User,
} from "../gen/orangevault_admin/v1/admin_pb.js";

type Tab = "users" | "orgs";

export function App() {
  const [token, setToken] = useState<string | null>(() => loadAdminToken());
  const [tab, setTab] = useState<Tab>("users");

  useEffect(() => {
    setAdminToken(token);
  }, [token]);

  if (!token) {
    return <TokenGate onSubmit={setToken} />;
  }

  return (
    <div className="shell">
      <header className="topbar">
        <span className="brand">orangevault • admin</span>
        <nav className="topnav">
          <button
            type="button"
            className={tab === "users" ? "tab active" : "tab"}
            onClick={() => setTab("users")}
          >
            Users
          </button>
          <button
            type="button"
            className={tab === "orgs" ? "tab active" : "tab"}
            onClick={() => setTab("orgs")}
          >
            Organizations
          </button>
          <span className="sep" />
          <button type="button" className="ghost" onClick={() => setToken(null)}>
            Forget token
          </button>
        </nav>
      </header>
      <main>{tab === "users" ? <UsersPanel /> : <OrgsPanel />}</main>
    </div>
  );
}

function TokenGate({ onSubmit }: { onSubmit: (token: string) => void }) {
  const [value, setValue] = useState("");
  return (
    <div className="gate">
      <h1>orangevault admin</h1>
      <p>
        Paste your <code>ADMIN_TOKEN</code> (the value of{" "}
        <code>fnox get ORANGEVAULT_ADMIN_TOKEN</code>):
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (value.trim()) onSubmit(value.trim());
        }}
      >
        <input
          type="password"
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="bearer token…"
        />
        <button type="submit">Unlock</button>
      </form>
    </div>
  );
}

function UsersPanel() {
  const [users, setUsers] = useState<User[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<User | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await adminClient.listUsers({ limit: 200 });
      setUsers(res.users);
    } catch (e) {
      setError(errorMessage(e, "ListUsers failed"));
    }
  }, []);
  useEffect(() => {
    load();
  }, [load]);

  if (error) return <ErrorBanner msg={error} onRetry={load} />;
  if (!users) return <p className="muted">Loading users…</p>;

  return (
    <section>
      <div className="panel-header">
        <h2>Users ({users.length})</h2>
        <button type="button" className="ghost" onClick={load}>
          Refresh
        </button>
      </div>
      <table>
        <thead>
          <tr>
            <th>Email</th>
            <th>Name</th>
            <th>Verified</th>
            <th>Created</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {users.map((u) => (
            <tr key={u.id}>
              <td>
                <code>{u.email}</code>
              </td>
              <td>{u.name}</td>
              <td>{u.emailVerified ? "✓" : "—"}</td>
              <td className="muted">{u.createdAt}</td>
              <td>
                <button type="button" className="ghost" onClick={() => setSelected(u)}>
                  Inspect
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selected && <UserDetail user={selected} onClose={() => setSelected(null)} onChange={load} />}
    </section>
  );
}

function UserDetail({
  user,
  onClose,
  onChange,
}: {
  user: User;
  onClose: () => void;
  onChange: () => void;
}) {
  const [memberships, setMemberships] = useState<Membership[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    adminClient
      .listUserMemberships({ userId: user.id })
      .then((res) => {
        if (!cancelled) setMemberships(res.memberships);
      })
      .catch((e) => {
        if (!cancelled) setError(errorMessage(e, "ListUserMemberships failed"));
      });
    return () => {
      cancelled = true;
    };
  }, [user.id]);

  const rotate = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await adminClient.rotateSecurityStamp({ userId: user.id });
      onChange();
      onClose();
    } catch (e) {
      setError(errorMessage(e, "RotateSecurityStamp failed"));
    } finally {
      setBusy(false);
    }
  }, [user.id, onChange, onClose]);

  return (
    <div className="modal" role="dialog">
      <div className="modal-card">
        <h3>{user.email}</h3>
        <dl>
          <dt>id</dt>
          <dd>
            <code>{user.id}</code>
          </dd>
          <dt>name</dt>
          <dd>{user.name}</dd>
          <dt>created</dt>
          <dd className="muted">{user.createdAt}</dd>
          <dt>updated</dt>
          <dd className="muted">{user.updatedAt}</dd>
        </dl>

        <h4>Memberships</h4>
        {memberships === null ? (
          <p className="muted">Loading…</p>
        ) : memberships.length === 0 ? (
          <p className="muted">No org memberships.</p>
        ) : (
          <ul>
            {memberships.map((m) => (
              <li key={m.organizationId}>
                <code>{m.organizationName}</code> — role {m.role}, status {m.status}
              </li>
            ))}
          </ul>
        )}

        {error && <p className="error">{error}</p>}

        <div className="actions">
          <button type="button" className="danger" disabled={busy} onClick={rotate}>
            Rotate security stamp (logs them out everywhere)
          </button>
          <button type="button" className="ghost" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function OrgsPanel() {
  const [orgs, setOrgs] = useState<Organization[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await adminClient.listOrganizations({});
      setOrgs(res.organizations);
    } catch (e) {
      setError(errorMessage(e, "ListOrganizations failed"));
    }
  }, []);
  useEffect(() => {
    load();
  }, [load]);

  if (error) return <ErrorBanner msg={error} onRetry={load} />;
  if (!orgs) return <p className="muted">Loading orgs…</p>;

  if (orgs.length === 0) {
    return (
      <p className="muted">
        No organizations yet. Create one from any Bitwarden client connected to
        orangevault.
      </p>
    );
  }
  return (
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Billing email</th>
          <th>Members</th>
          <th>Id</th>
        </tr>
      </thead>
      <tbody>
        {orgs.map((o) => (
          <tr key={o.id}>
            <td>{o.name}</td>
            <td>
              <code>{o.billingEmail}</code>
            </td>
            <td>{o.memberCount}</td>
            <td>
              <code className="muted">{o.id}</code>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ErrorBanner({ msg, onRetry }: { msg: string; onRetry: () => void }) {
  return (
    <div className="error-banner">
      <span>{msg}</span>
      <button type="button" onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}
