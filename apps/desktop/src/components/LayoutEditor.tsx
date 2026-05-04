import { useMemo, useState } from "react";

type Direction = "left" | "right" | "top" | "bottom";
type LayoutMap = Record<Direction, string>;

const DIR_LABEL: Record<Direction, string> = {
  left: "Left edge",
  right: "Right edge",
  top: "Top edge",
  bottom: "Bottom edge"
};

type Props = {
  peers: string[];
  onLayoutChange: (layout: LayoutMap) => void;
};

export default function LayoutEditor({ peers, onLayoutChange }: Props) {
  const [layout, setLayout] = useState<LayoutMap>({
    left: "",
    right: "",
    top: "",
    bottom: ""
  });

  const options = useMemo(() => ["", ...peers], [peers]);

  const update = (direction: Direction, value: string) => {
    const next = { ...layout, [direction]: value };
    setLayout(next);
    onLayoutChange(next);
  };

  return (
    <section className="panel" aria-labelledby="layout-panel-title">
      <div className="panel__header">
        <h3 id="layout-panel-title" className="panel__title">
          Edge mappings
        </h3>
        <p className="panel__hint">Peer that receives control when the cursor exits each edge</p>
      </div>
      <div className="panel__body">
        <div className="layout-grid">
          {(["left", "right", "top", "bottom"] as Direction[]).map((direction) => (
            <div key={direction} className="field" style={{ marginBottom: 0 }}>
              <label className="field__label" htmlFor={`edge-${direction}`}>
                {DIR_LABEL[direction]}
              </label>
              <select
                id={`edge-${direction}`}
                value={layout[direction]}
                onChange={(event) => update(direction, event.target.value)}
              >
                {options.map((peer) => (
                  <option key={peer || "none"} value={peer}>
                    {peer || "None"}
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
