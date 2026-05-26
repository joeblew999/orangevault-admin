import { useEffect, useState } from "react";
import { Link, Navigate, Route, Routes, useLocation } from "react-router-dom";
import { loadAdminToken, setAdminToken } from "./client";
import { Users } from "./pages/Users";
import { UserDetail } from "./pages/UserDetail";
import { Orgs } from "./pages/Orgs";
import { TokenGate } from "./pages/TokenGate";

export function App() {
  const [token, setToken] = useState<string | null>(() => loadAdminToken());
  const loc = useLocation();

  useEffect(() => {
    setAdminToken(token);
  }, [token]);

  if (!token) {
    return (
      <div className="shell">
        <TokenShell />
        <main>
          <TokenGate onSubmit={setToken} />
        </main>
      </div>
    );
  }

  return (
    <div className="shell">
      <header className="topbar">
        <Link to="/" className="brand">
          <span className="dot" aria-hidden />
          <span>orangevault // admin</span>
          <span className="ver">v0.1.0</span>
        </Link>
        <nav className="topnav">
          <SystemStatus />
          <span className="sep" aria-hidden />
          <Link
            to="/"
            aria-current={
              loc.pathname === "/" || loc.pathname.startsWith("/users") ? "page" : undefined
            }
          >
            Users
          </Link>
          <Link to="/orgs" aria-current={loc.pathname === "/orgs" ? "page" : undefined}>
            Orgs
          </Link>
          <button className="ghost" type="button" onClick={() => setToken(null)}>
            Forget token
          </button>
        </nav>
      </header>

      <main>
        <Routes>
          <Route path="/" element={<Users />} />
          <Route path="/users/:id" element={<UserDetail />} />
          <Route path="/orgs" element={<Orgs />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>

      <footer className="footer-caption">
        <span>orangevault // admin</span>
        <span>Connect-RPC · Cloudflare Workers</span>
      </footer>
    </div>
  );
}

function TokenShell() {
  return (
    <header className="topbar">
      <span className="brand">
        <span className="dot" aria-hidden />
        <span>orangevault // admin</span>
        <span className="ver">locked</span>
      </span>
      <nav className="topnav">
        <SystemStatus />
      </nav>
    </header>
  );
}

function SystemStatus() {
  const [now, setNow] = useState(() => formatClock(new Date()));
  useEffect(() => {
    let id: number;
    function tick() {
      setNow(formatClock(new Date()));
      id = window.setTimeout(tick, 1000 - (Date.now() % 1000));
    }
    id = window.setTimeout(tick, 1000 - (Date.now() % 1000));
    return () => window.clearTimeout(id);
  }, []);
  return (
    <span className="status" title="Live · UTC">
      <span className="signal" aria-hidden />
      <span>Live · {now}</span>
    </span>
  );
}

function formatClock(d: Date): string {
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  const ss = String(d.getUTCSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}Z`;
}
