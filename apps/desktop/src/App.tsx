import { useEffect, useMemo, useState } from "react";
import LayoutEditor from "./components/LayoutEditor";
import PairingFlow from "./components/PairingFlow";
import {
  configureLayout,
  connectToHost,
  evaluateEdgeHandoff,
  failSafeRelease,
  heartbeat,
  healthCheck,
  networkStatus,
  injectTestInput,
  relayClipboardText,
  startInputCapture,
  startHostListener,
  trustedPeers
} from "./lib/tauri";

type Role = "host" | "client";

export default function App() {
  const [role, setRole] = useState<Role>("host");
  const [status, setStatus] = useState("Idle");
  const [peers, setPeers] = useState<string[]>([]);
  const [clipboardText, setClipboardText] = useState("");
  const [layout, setLayout] = useState<Record<string, string>>({});
  const [hostAddr, setHostAddr] = useState("0.0.0.0:24800");
  const [remoteAddr, setRemoteAddr] = useState("127.0.0.1:24800");
  const [pairCodeForConnect, setPairCodeForConnect] = useState("");

  const activeMappings = useMemo(
    () => Object.entries(layout).filter(([, peer]) => Boolean(peer)),
    [layout]
  );

  const reloadPeers = async () => {
    const list = await trustedPeers();
    setPeers(list.map((item) => item.name));
  };

  useEffect(() => {
    void reloadPeers();
    void healthCheck().then(() => setStatus("Ready"));

    const interval = window.setInterval(() => {
      void heartbeat()
        .then(() => setStatus((prev) => (prev.startsWith("Disconnected") ? "Reconnected" : prev)))
        .catch(() => setStatus("Disconnected. Retrying heartbeat..."));
      void networkStatus().then((s) => {
        if (s.last_error) {
          setStatus(`Network error: ${s.last_error}`);
        }
      });
    }, 2500);

    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.altKey && event.key.toLowerCase() === "q") {
        void failSafeRelease().then(() => setStatus("Emergency release hotkey used"));
      }
    };

    window.addEventListener("keydown", onKey);
    return () => {
      clearInterval(interval);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const syncClipboard = async () => {
    const revision = await relayClipboardText({
      text: clipboardText,
      sourcePeer: "00000000-0000-0000-0000-000000000000"
    });
    setStatus(`Clipboard synced, revision ${revision}`);
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

  return (
    <main>
      <h1>Lan Input Sync</h1>
      <p className="muted">Role mode, secure pairing, edge handoff, and text clipboard sync.</p>

      <section className="card">
        <h2>Device Role</h2>
        <div className="row">
          <button onClick={() => setRole("host")} disabled={role === "host"}>
            Host
          </button>
          <button onClick={() => setRole("client")} disabled={role === "client"}>
            Client
          </button>
        </div>
        <p>Current role: {role.toUpperCase()}</p>
      </section>

      <PairingFlow onPaired={reloadPeers} />

      <section className="card">
        <h2>Network Session</h2>
        <div className="stack">
          <input value={hostAddr} onChange={(e) => setHostAddr(e.target.value)} />
          <button
            onClick={() =>
              startHostListener(hostAddr).then((s) =>
                setStatus(`Hosting on ${s.listening_addr || hostAddr}`)
              )
            }
          >
            Start Host Listener
          </button>
        </div>
        <hr />
        <div className="stack">
          <input value={remoteAddr} onChange={(e) => setRemoteAddr(e.target.value)} />
          <input
            value={pairCodeForConnect}
            onChange={(e) => setPairCodeForConnect(e.target.value.toUpperCase())}
            placeholder="Pairing code"
          />
          <button
            onClick={() =>
              connectToHost(remoteAddr, pairCodeForConnect, "Client Device").then((s) =>
                setStatus(`Connected to ${s.connected_peer || remoteAddr}`)
              )
            }
          >
            Connect to Host
          </button>
        </div>
      </section>
      <LayoutEditor peers={peers} onLayoutChange={setLayout} />

      <section className="card">
        <h2>Clipboard</h2>
        <textarea
          value={clipboardText}
          onChange={(e) => setClipboardText(e.target.value)}
          placeholder="Text to sync between devices"
          rows={4}
        />
        <button onClick={syncClipboard}>Sync Clipboard Text</button>
      </section>

      <section className="card">
        <h2>Session Control</h2>
        <div className="row">
          <button onClick={testEdge}>Test Edge Handoff</button>
          <button onClick={() => startInputCapture().then(() => setStatus("Input capture started"))}>
            Start Input Capture
          </button>
          <button onClick={() => injectTestInput(500, 300, 0x41).then(() => setStatus("Injected test input"))}>
            Inject Test Input
          </button>
          <button onClick={() => failSafeRelease().then(() => setStatus("Local control restored"))}>
            Emergency Release
          </button>
        </div>
        {activeMappings.length > 0 ? (
          <p>
            Active mappings:{" "}
            {activeMappings.map(([dir, peer]) => `${dir}→${peer}`).join(", ")}
          </p>
        ) : (
          <p>No edge mappings configured yet.</p>
        )}
      </section>

      <p className="status">{status}</p>
    </main>
  );
}
