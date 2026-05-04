import { useEffect, useMemo, useState } from "react";
import LayoutEditor from "./components/LayoutEditor";
import {
  configureLayout,
  connectToHost,
  evaluateEdgeHandoff,
  failSafeRelease,
  heartbeat,
  healthCheck,
  localDeviceName,
  nearbyPeers,
  networkStatus,
  injectTestInput,
  startInputCapture,
  startSharing,
  trustedPeers,
  type NearbyPeer,
  type NetworkStatus
} from "./lib/tauri";

type NavId = "connect" | "layout" | "developer";

export default function App() {
  const [nav, setNav] = useState<NavId>("connect");
  const [status, setStatus] = useState("Idle");
  const [peers, setPeers] = useState<string[]>([]);
  const [layout, setLayout] = useState<Record<string, string>>({});
  const [thisDeviceName, setThisDeviceName] = useState("This computer");
  const [nearby, setNearby] = useState<NearbyPeer[]>([]);
  const [sharingCode, setSharingCode] = useState<string | null>(null);
  const [network, setNetwork] = useState<NetworkStatus | null>(null);
  const [pairCodeToJoin, setPairCodeToJoin] = useState("");

  const activeMappings = useMemo(
    () => Object.entries(layout).filter(([, peer]) => Boolean(peer)),
    [layout]
  );

  const reloadPeers = async () => {
    const list = await trustedPeers();
    setPeers(list.map((item) => item.name));
  };

  useEffect(() => {
    void localDeviceName().then(setThisDeviceName);
    void reloadPeers();
    void healthCheck().then(() => setStatus("Ready"));

    const interval = window.setInterval(() => {
      void heartbeat()
        .then(() => setStatus((prev) => (prev.startsWith("Disconnected") ? "Reconnected" : prev)))
        .catch(() => setStatus("Disconnected. Retrying heartbeat..."));
      void networkStatus().then((s) => {
        setNetwork(s);
        if (s.last_error) {
          setStatus(`Network error: ${s.last_error}`);
        }
      });
    }, 2500);

    const discoveryInterval = window.setInterval(() => {
      void nearbyPeers().then(setNearby);
    }, 2000);
    void nearbyPeers().then(setNearby);

    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.altKey && event.key.toLowerCase() === "q") {
        void failSafeRelease().then(() => setStatus("Emergency release hotkey used"));
      }
    };

    window.addEventListener("keydown", onKey);
    return () => {
      clearInterval(interval);
      clearInterval(discoveryInterval);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const beginSharing = async () => {
    try {
      const res = await startSharing();
      setSharingCode(res.pairing_code);
      setNetwork(res.status);
      setStatus("Sharing — tell the other PC this code, then connect from there.");
    } catch (e) {
      setStatus(`Could not start sharing: ${String(e)}`);
    }
  };

  const joinPeer = async (peer: NearbyPeer) => {
    const code = pairCodeToJoin.trim();
    if (!code) {
      setStatus("Enter the pairing code shown on the other computer.");
      return;
    }
    try {
      const s = await connectToHost(peer.address, code.toUpperCase(), thisDeviceName);
      setNetwork(s);
      setStatus(`Linked with ${peer.name}.`);
    } catch (e) {
      setStatus(`Could not connect: ${String(e)}`);
    }
  };

  const testEdge = async () => {
    const edge = await evaluateEdgeHandoff({
      currentX: window.innerWidth - 1,
      currentY: Math.floor(window.innerHeight / 2),
      desktopWidth: window.innerWidth,
      desktopHeight: window.innerHeight
    });
    setStatus(edge ? `Edge handoff detected: ${edge}` : "No handoff");
  };

  useEffect(() => {
    void configureLayout({
      left: layout.left || undefined,
      right: layout.right || undefined,
      top: layout.top || undefined,
      bottom: layout.bottom || undefined
    });
  }, [layout]);

  const linked = Boolean(network?.connected_peer);

  const connectionPill = useMemo(() => {
    if (network?.last_error) {
      return { className: "pill pill--bad", label: "Network issue", detail: network.last_error };
    }
    if (linked) {
      return {
        className: "pill pill--ok",
        label: "Linked",
        detail: network?.connected_peer ?? ""
      };
    }
    if (sharingCode) {
      return { className: "pill pill--warn", label: "Sharing", detail: "Waiting for other PC" };
    }
    return { className: "pill", label: "Not linked", detail: "Start sharing or connect below" };
  }, [linked, network?.connected_peer, network?.last_error, sharingCode]);

  const statusBarMods = useMemo(() => {
    const parts = ["status-bar"];
    if (network?.last_error || status.startsWith("Disconnected")) parts.push("status-bar--err");
    else if (linked || sharingCode || status === "Ready") parts.push("status-bar--live");
    return parts.join(" ");
  }, [linked, network?.last_error, sharingCode, status]);

  return (
    <div className="app">
      <aside className="app__sidebar" aria-label="Main navigation">
        <div className="brand">
          <div className="brand__mark" aria-hidden />
          <h1 className="brand__title">Sync</h1>
          <div className="brand__version">Version 0.1.0</div>
        </div>
        <nav className="nav">
          <button
            type="button"
            className={`nav__btn ${nav === "connect" ? "nav__btn--active" : ""}`}
            onClick={() => setNav("connect")}
          >
            <span className="nav__icon" aria-hidden>
              ◎
            </span>
            Connect
          </button>
          <button
            type="button"
            className={`nav__btn ${nav === "layout" ? "nav__btn--active" : ""}`}
            onClick={() => setNav("layout")}
          >
            <span className="nav__icon" aria-hidden>
              ⊞
            </span>
            Screen layout
          </button>
          <button
            type="button"
            className={`nav__btn ${nav === "developer" ? "nav__btn--active" : ""}`}
            onClick={() => setNav("developer")}
          >
            <span className="nav__icon" aria-hidden>
              ⚙
            </span>
            Developer
          </button>
        </nav>
      </aside>

      <div className="app__body">
        <main className="app__main">
          <div className="app__main-inner">
            {nav === "connect" && (
              <>
                <header className="page-head">
                  <div className="toolbar">
                    <div>
                      <h2 className="page-head__title">Connection</h2>
                      <p className="page-head__desc">
                        Discover other PCs on your network, confirm with a short code, then copy and paste across
                        machines.
                      </p>
                    </div>
                    <div className={connectionPill.className} title={connectionPill.detail}>
                      <span className="pill__dot" />
                      <span>{connectionPill.label}</span>
                    </div>
                  </div>
                </header>

                <section className="panel" aria-labelledby="panel-link-title">
                  <div className="panel__header">
                    <h3 id="panel-link-title" className="panel__title">
                      This computer
                    </h3>
                  </div>
                  <div className="panel__body">
                    <div className="device-line">
                      <span className="device-line__label">Device</span>
                      <span className="device-line__name">{thisDeviceName}</span>
                    </div>
                  </div>
                </section>

                <section className="panel" aria-labelledby="panel-share-title">
                  <div className="panel__header">
                    <h3 id="panel-share-title" className="panel__title">
                      Share or join
                    </h3>
                    <p className="panel__hint">One side shares a code; the other enters it and connects.</p>
                  </div>
                  <div className="panel__body">
                    <div className="btn-row" style={{ marginBottom: 14 }}>
                      <button type="button" className="btn btn--primary" onClick={() => void beginSharing()}>
                        Start sharing
                      </button>
                      <span className="muted" style={{ fontSize: 12 }}>
                        Shows a pairing code others can use for a few minutes.
                      </span>
                    </div>

                    {sharingCode ? (
                      <div className="share-panel">
                        <p className="share-panel__label">Pairing code</p>
                        <p className="pairing-code">{sharingCode}</p>
                      </div>
                    ) : null}

                    <div className="field" style={{ marginTop: sharingCode ? 18 : 0 }}>
                      <label className="field__label" htmlFor="join-code">
                        Code from the other computer (when joining)
                      </label>
                      <input
                        id="join-code"
                        value={pairCodeToJoin}
                        onChange={(e) => setPairCodeToJoin(e.target.value.toUpperCase())}
                        placeholder="e.g. ABC123"
                        autoCapitalize="characters"
                        spellCheck={false}
                      />
                    </div>
                  </div>
                </section>

                <section className="panel" aria-labelledby="panel-nearby-title">
                  <div className="panel__header">
                    <h3 id="panel-nearby-title" className="panel__title">
                      Nearby devices
                    </h3>
                    <p className="panel__hint">Same Wi‑Fi · scans every few seconds</p>
                  </div>
                  <div className="panel__body">
                    {nearby.length === 0 ? (
                      <div className="empty-state">
                        <p className="empty-state__title">No devices found</p>
                        <p style={{ margin: 0 }}>
                          Open this app on another machine on the same network, or check firewall settings for local
                          traffic.
                        </p>
                      </div>
                    ) : (
                      <ul className="peer-list">
                        {nearby.map((p) => (
                          <li key={p.id} className="peer-row">
                            <div className="peer-avatar" aria-hidden>
                              {p.name.trim().charAt(0).toUpperCase() || "?"}
                            </div>
                            <div className="peer-meta">
                              <div className="peer-name">{p.name}</div>
                              <div className="peer-addr">{p.address}</div>
                            </div>
                            <button type="button" className="btn btn--secondary btn--sm" onClick={() => void joinPeer(p)}>
                              Connect
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </section>

                <section className="panel" aria-labelledby="panel-clip-title">
                  <div className="panel__header">
                    <h3 id="panel-clip-title" className="panel__title">
                      Clipboard
                    </h3>
                  </div>
                  <div className="panel__body">
                    <p style={{ margin: 0, color: "var(--text-secondary)", fontSize: 13 }}>
                      When linked{linked ? "" : " (after you connect)"}, text you copy with <kbd>Ctrl</kbd>+<kbd>C</kbd>{" "}
                      on one PC appears on the other—use <kbd>Ctrl</kbd>+<kbd>V</kbd> as usual. No extra sync button.
                    </p>
                    <div className="callout callout--tip">
                      <span className="callout__icon" aria-hidden>
                        ⏎
                      </span>
                      <span>
                        Tip: keep both apps running while you work. Link status appears in the header and status bar.
                      </span>
                    </div>
                  </div>
                </section>
              </>
            )}

            {nav === "layout" && (
              <>
                <header className="page-head">
                  <h2 className="page-head__title">Screen layout</h2>
                  <p className="page-head__desc">
                    Map screen edges to peers so the cursor can hand off control when you move past an edge.
                  </p>
                </header>
                <LayoutEditor peers={peers} onLayoutChange={setLayout} />
              </>
            )}

            {nav === "developer" && (
              <>
                <header className="page-head">
                  <h2 className="page-head__title">Developer</h2>
                  <p className="page-head__desc">
                    Input capture, injection tests, and emergency release. Use when debugging keyboard and mouse sync.
                  </p>
                </header>

                <section className="panel">
                  <div className="panel__header">
                    <h3 className="panel__title">Diagnostics</h3>
                  </div>
                  <div className="panel__body">
                    <div className="btn-row">
                      <button type="button" className="btn btn--secondary" onClick={() => void testEdge()}>
                        Test edge handoff
                      </button>
                      <button
                        type="button"
                        className="btn btn--secondary"
                        onClick={() => void startInputCapture().then(() => setStatus("Input capture started"))}
                      >
                        Start input capture
                      </button>
                      <button
                        type="button"
                        className="btn btn--secondary"
                        onClick={() => void injectTestInput(500, 300, 0x41).then(() => setStatus("Injected test input"))}
                      >
                        Inject test input
                      </button>
                    </div>
                  </div>
                </section>

                <section className="panel danger-zone">
                  <div className="panel__header">
                    <h3 className="panel__title">Safety</h3>
                    <p className="panel__hint">If control feels stuck</p>
                  </div>
                  <div className="panel__body">
                    <p style={{ margin: "0 0 12px", color: "var(--text-secondary)", fontSize: 13 }}>
                      Restores local keyboard and mouse immediately. Shortcut: <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+
                      <kbd>Q</kbd>
                    </p>
                    <button type="button" className="btn btn--danger" onClick={() => void failSafeRelease().then(() => setStatus("Local control restored"))}>
                      Emergency release
                    </button>
                  </div>
                </section>

                {activeMappings.length > 0 ? (
                  <p className="muted" style={{ fontSize: 12, marginTop: 8 }}>
                    Active edge mappings: {activeMappings.map(([dir, peer]) => `${dir}→${peer}`).join(", ")}
                  </p>
                ) : null}
              </>
            )}
          </div>
        </main>

        <footer className={statusBarMods} role="status">
          <span className="status-bar__dot" aria-hidden />
          <span>{status}</span>
        </footer>
      </div>
    </div>
  );
}
