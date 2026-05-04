import { useMemo, useState } from "react";

type Direction = "left" | "right" | "top" | "bottom";
type LayoutMap = Record<Direction, string>;

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
    <section className="card">
      <h2>Screen Layout</h2>
      <p>Choose which peer should receive control when your cursor exits an edge.</p>
      <div className="grid">
        {(["left", "right", "top", "bottom"] as Direction[]).map((direction) => (
          <label key={direction} className="stack">
            <span>{direction.toUpperCase()}</span>
            <select
              value={layout[direction]}
              onChange={(event) => update(direction, event.target.value)}
            >
              {options.map((peer) => (
                <option key={peer || "none"} value={peer}>
                  {peer || "No mapping"}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
    </section>
  );
}
