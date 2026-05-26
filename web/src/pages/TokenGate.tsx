import { useState } from "react";

export function TokenGate({ onSubmit }: { onSubmit: (token: string) => void }) {
  const [value, setValue] = useState("");
  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">locked</span>
        <h1 className="display sm">Paste admin token</h1>
        <p className="lede">
          The value of <code className="mono">fnox get ORANGEVAULT_ADMIN_TOKEN</code> on
          the operator's machine. Stored in localStorage on this device only.
        </p>
      </section>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const v = value.trim();
          if (v) onSubmit(v);
        }}
        className="form-card"
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
