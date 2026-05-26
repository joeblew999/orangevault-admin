import { useCallback, useEffect, useState } from "react";
import { adminClient, errorMessage } from "../client";
import type { Organization } from "../../gen/orangevault_admin/v1/admin_pb.js";

export function Orgs() {
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

  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">scope · all orgs</span>
        <h1 className="display sm">Organizations</h1>
        <p className="lede">
          Every org on this orangevault server. Orgs are created by users via the
          standard Bitwarden client flow.
        </p>
      </section>

      <section>
        <div className="section-head">
          <h2>
            {orgs === null
              ? "Loading…"
              : `${orgs.length} organization${orgs.length === 1 ? "" : "s"}`}
          </h2>
          <button type="button" className="ghost" onClick={load}>
            Refresh
          </button>
        </div>

        {error && <p className="status-line error">{error}</p>}

        {orgs && orgs.length === 0 && (
          <p className="status-line">No organizations yet.</p>
        )}

        {orgs && orgs.length > 0 && (
          <table className="data-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Billing email</th>
                <th>Members</th>
                <th>ID</th>
              </tr>
            </thead>
            <tbody>
              {orgs.map((o) => (
                <tr key={o.id}>
                  <td>{o.name}</td>
                  <td className="mono">{o.billingEmail}</td>
                  <td>{o.memberCount}</td>
                  <td className="mono secondary">{o.id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
