import { FormEvent, useState } from "react";
import { generatePairingCode, submitPairing } from "../lib/tauri";

type Props = {
  onPaired: () => void;
};

export default function PairingFlow({ onPaired }: Props) {
  const [deviceName, setDeviceName] = useState("");
  const [pairingCode, setPairingCode] = useState("");
  const [generatedCode, setGeneratedCode] = useState("");
  const [status, setStatus] = useState<string>("");

  const createCode = async () => {
    const code = await generatePairingCode();
    setGeneratedCode(code);
    setStatus("Pairing code generated. Use it on the peer machine within 5 minutes.");
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setStatus("Pairing in progress...");
    const res = await submitPairing({
      deviceName: deviceName || "Unnamed Device",
      pairingCode,
      certFingerprint: "demo-fingerprint"
    });
    if (res.accepted) {
      setStatus("Paired successfully.");
      onPaired();
    } else {
      setStatus(res.reason || "Pairing failed.");
    }
  };

  return (
    <section className="card">
      <h2>Pair Devices</h2>
      <button onClick={createCode}>Generate Pairing Code (Host)</button>
      {generatedCode ? <p className="code">{generatedCode}</p> : null}
      <form onSubmit={handleSubmit} className="stack">
        <input
          value={deviceName}
          onChange={(e) => setDeviceName(e.target.value)}
          placeholder="This device name"
        />
        <input
          value={pairingCode}
          onChange={(e) => setPairingCode(e.target.value.toUpperCase())}
          placeholder="Pairing code from host"
          required
        />
        <button type="submit">Trust Peer</button>
      </form>
      {status ? <p>{status}</p> : null}
    </section>
  );
}
