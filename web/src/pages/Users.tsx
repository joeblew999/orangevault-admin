import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { adminClient, errorMessage } from "../client";
import type { User } from "../../gen/orangevault_admin/v1/admin_pb.js";

export function Users() {
  const [users, setUsers] = useState<User[] | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">scope · all users</span>
        <h1 className="display sm">Users</h1>
        <p className="lede">
          Every account on this orangevault server. Click a row to inspect, rotate
          security stamp, or hard-delete with cascade.
        </p>
      </section>

      <section>
        <div className="section-head">
          <h2>
            {users === null ? "Loading…" : `${users.length} user${users.length === 1 ? "" : "s"}`}
          </h2>
          <button type="button" className="ghost" onClick={load}>
            Refresh
          </button>
        </div>

        {error && <p className="status-line error">{error}</p>}

        {users && users.length > 0 && (
          <table className="data-table">
            <thead>
              <tr>
                <th>Email</th>
                <th>Name</th>
                <th>Verified</th>
                <th>Created (UTC)</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => (
                <tr key={u.id}>
                  <td>
                    <Link to={`/users/${u.id}`} className="mono">
                      {u.email}
                    </Link>
                  </td>
                  <td>{u.name}</td>
                  <td>
                    {u.emailVerified ? (
                      <span className="chip active">verified</span>
                    ) : (
                      <span className="chip">unverified</span>
                    )}
                  </td>
                  <td className="mono secondary">{u.createdAt}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
