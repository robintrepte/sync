import { invoke } from "@tauri-apps/api/core";

export type TrustedPeer = {
  peer_id: string;
  name: string;
  fingerprint: string;
  trusted_at_ms: number;
};

export type PairingResponse = {
  accepted: boolean;
  peer_id?: string;
  reason?: string;
};

export type NetworkStatus = {
  listening_addr?: string;
  connected_peer?: string;
  last_error?: string;
};

export async function generatePairingCode(): Promise<string> {
  return invoke("generate_pairing_code");
}

export async function submitPairing(payload: {
  deviceName: string;
  pairingCode: string;
  certFingerprint: string;
}): Promise<PairingResponse> {
  return invoke("submit_pairing", {
    deviceName: payload.deviceName,
    pairingCode: payload.pairingCode,
    certFingerprint: payload.certFingerprint
  });
}

export async function trustedPeers(): Promise<TrustedPeer[]> {
  return invoke("trusted_peers");
}

export async function relayClipboardText(payload: {
  text: string;
  sourcePeer: string;
}): Promise<number> {
  return invoke("relay_clipboard_text", {
    req: {
      text: payload.text,
      source_peer: payload.sourcePeer
    }
  });
}

export async function evaluateEdgeHandoff(payload: {
  currentX: number;
  currentY: number;
  desktopWidth: number;
  desktopHeight: number;
}): Promise<string | null> {
  return invoke("evaluate_edge_handoff", {
    req: {
      current_x: payload.currentX,
      current_y: payload.currentY,
      desktop_width: payload.desktopWidth,
      desktop_height: payload.desktopHeight
    }
  });
}

export async function configureLayout(layout: {
  left?: string;
  right?: string;
  top?: string;
  bottom?: string;
}): Promise<void> {
  return invoke("configure_layout", { req: layout });
}

export async function failSafeRelease(): Promise<void> {
  return invoke("fail_safe_release");
}

export async function healthCheck(): Promise<string> {
  return invoke("health_check");
}

export async function heartbeat(): Promise<number> {
  return invoke("heartbeat");
}

export async function startHostListener(bindAddr: string): Promise<NetworkStatus> {
  return invoke("start_host_listener", { bindAddr });
}

export async function connectToHost(
  hostAddr: string,
  pairingCode: string,
  deviceName: string
): Promise<NetworkStatus> {
  return invoke("connect_to_host", { hostAddr, pairingCode, deviceName });
}

export async function networkStatus(): Promise<NetworkStatus> {
  return invoke("network_status");
}

export async function startInputCapture(): Promise<void> {
  return invoke("start_input_capture");
}

export async function injectTestInput(dx: number, dy: number, keyCode: number): Promise<void> {
  return invoke("inject_test_input", { dx, dy, keyCode });
}
