import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { adminClient, errorMessage } from "../client";
import type { Membership, User } from "../../gen/orangevault_admin/v1/admin_pb.js";

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; user: User; memberships: Membership[] }
  | { kind: "error"; message: string };

export function UserDetail() {
  const { id } = useParams<{ id: string }>();
  const nav = useNavigate();
  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState<"rotate" | "delete" | null>(null);
  const [confirmEmail, setConfirmEmail] = useState("");

  const refresh = useCallback(async () => {
    if (!id) {
      setLoad({ kind: "error", message: "missing id" });
      return;
    }
    setLoad({ kind: "loading" });
    try {
      const [u, m] = await Promise.all([
        adminClient.getUser({ userId: id }),
        adminClient.listUserMemberships({ userId: id }),
      ]);
      if (!u.user) throw new Error("user missing in response");
      setLoad({ kind: "loaded", user: u.user, memberships: m.memberships });
    } catch (e) {
      setLoad({ kind: "error", message: errorMessage(e, "load failed") });
    }
  }, [id]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const rotate = useCallback(async () => {
    if (load.kind !== "loaded") return;
    setActionError(null);
    setBusy("rotate");
    try {
      await adminClient.rotateSecurityStamp({ userId: load.user.id });
      await refresh();
    } catch (e) {
      setActionError(errorMessage(e, "RotateSecurityStamp failed"));
    } finally {
      setBusy(null);
    }
  }, [load, refresh]);

  const remove = useCallback(async () => {
    if (load.kind !== "loaded") return;
    setActionError(null);
    setBusy("delete");
    try {
      const res = await adminClient.deleteUser({ userId: load.user.id });
      console.info(`deleted ${load.user.email}, rows=${res.deletedRows}`);
      nav("/");
    } catch (e) {
      setActionError(errorMessage(e, "DeleteUser failed"));
      setBusy(null);
    }
  }, [load, nav]);

  if (load.kind === "loading") return <p className="status-line">Loading user…</p>;
  if (load.kind === "error")
    return (
      <div className="page">
        <p className="status-line error">{load.message}</p>
        <p>
          <Link to="/">← back to users</Link>
        </p>
      </div>
    );

  const { user, memberships } = load;
  const canDelete = busy !== "delete" && confirmEmail.trim() === user.email;

  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">user</span>
        <h1 className="display sm">{user.email}</h1>
        <p className="lede">
          <Link to="/">← all users</Link>
        </p>
      </section>

      <section>
        <h2>Profile</h2>
        <dl className="kv">
          <div className="row">
            <dt>User ID</dt>
            <dd>
              <span className="mono">{user.id}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Name</dt>
            <dd>{user.name || <span className="secondary">—</span>}</dd>
          </div>
          <div className="row">
            <dt>Email verified</dt>
            <dd>
              {user.emailVerified ? (
                <span className="chip active">verified</span>
              ) : (
                <span className="chip">unverified</span>
              )}
            </dd>
          </div>
          <div className="row">
            <dt>Created</dt>
            <dd className="mono secondary">{user.createdAt}</dd>
          </div>
          <div className="row">
            <dt>Updated</dt>
            <dd className="mono secondary">{user.updatedAt}</dd>
          </div>
        </dl>
      </section>

      <section>
        <h2>Memberships</h2>
        {memberships.length === 0 ? (
          <p className="status-line">No org memberships.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Organization</th>
                <th>Role</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {memberships.map((m) => (
                <tr key={m.organizationId}>
                  <td>{m.organizationName}</td>
                  <td>{m.role}</td>
                  <td>{m.status}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Actions</h2>
        {actionError && <p className="status-line error">{actionError}</p>}
        <button type="button" className="ghost" disabled={busy !== null} onClick={rotate}>
          Rotate security stamp
        </button>
      </section>

      <details className="danger-zone">
        <summary>Danger zone</summary>
        <p className="lede">
          Hard-deletes the user and every row they own (devices, ciphers, folders,
          sends, 2FA, memberships). Irreversible. Type{" "}
          <span className="mono">{user.email}</span> to confirm.
        </p>
        <input
          type="text"
          placeholder={user.email}
          value={confirmEmail}
          onChange={(e) => setConfirmEmail(e.target.value)}
        />
        <button type="button" className="danger" disabled={!canDelete} onClick={remove}>
          Delete user
        </button>
      </details>
    </div>
  );
}
