export type WireMessage =
  | { kind: "PairingRequest"; payload: PairingRequest }
  | { kind: "PairingAck"; payload: PairingAck }
  | { kind: "InputEvent"; payload: InputEvent }
  | { kind: "ClipboardTextUpdate"; payload: ClipboardTextUpdate }
  | { kind: "FocusChange"; payload: FocusChange }
  | { kind: "Heartbeat"; payload: Heartbeat };

export type PairingRequest = {
  deviceName: string;
  pairingCode: string;
  certFingerprint: string;
};

export type PairingAck = {
  peerId: string;
  accepted: boolean;
  reason?: string;
};

export type InputEvent = {
  sequence: number;
  event:
    | { type: "MouseMove"; data: { dx: number; dy: number } }
    | { type: "MouseButton"; data: { button: number; down: boolean } }
    | { type: "MouseWheel"; data: { deltaX: number; deltaY: number } }
    | { type: "Key"; data: { code: number; down: boolean } };
};

export type ClipboardTextUpdate = {
  revision: number;
  sourcePeer: string;
  text: string;
};

export type FocusChange = {
  targetPeer: string;
  hasFocus: boolean;
};

export type Heartbeat = {
  timestampMs: number;
};
