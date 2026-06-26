import type { CSSProperties, ReactNode } from "react";

export type TooltipLine = { color?: string; label: string; value: string };
export type FloatingTooltipData = {
  color?: string;
  lines?: TooltipLine[];
  primary: string;
  secondary?: string;
  title: string;
  x: number;
  y: number;
};
export type TooltipControls = {
  hideTooltip: () => void;
  showTooltip: (tooltip: FloatingTooltipData) => void;
};

export function Panel({
  action,
  children,
  className,
  title,
}: {
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  title: string;
}) {
  return (
    <section className={["panel", className].filter(Boolean).join(" ")}>
      <div className="panel-header">
        <h2>{title}</h2>
        <div className="panel-action">{action}</div>
      </div>
      <div className="panel-body">{children}</div>
    </section>
  );
}

export function SwitchRow({
  checked,
  disabled,
  label,
  note,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  note?: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className={disabled ? "switch-row disabled" : "switch-row"}>
      <span>
        <strong>{label}</strong>
        {note ? <small>{note}</small> : null}
      </span>
      <input checked={checked} disabled={disabled} type="checkbox" onChange={(event) => onChange(event.currentTarget.checked)} />
      <em />
    </label>
  );
}

export function SegmentedControl<T extends number | string>({
  className,
  getTitle,
  onChange,
  options,
  value,
}: {
  className?: string;
  getTitle?: (value: T) => string;
  onChange: (value: T) => void;
  options: Array<{ icon?: ReactNode; label: ReactNode; value: T }>;
  value: T;
}) {
  return (
    <div className={["segmented-control", className].filter(Boolean).join(" ")}>
      {options.map((option) => (
        <button
          className={value === option.value ? "selected" : undefined}
          key={String(option.value)}
          type="button"
          onClick={() => onChange(option.value)}
          title={getTitle?.(option.value)}
        >
          {option.icon}
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function FloatingTooltip({ color, lines = [], primary, secondary, title, x, y }: FloatingTooltipData) {
  const targetWidth = Math.min(lines.length ? 330 : 240, window.innerWidth - 24);
  const placeRight = x + 18 + targetWidth < window.innerWidth;
  const left = placeRight ? x + 16 : Math.max(12, x - targetWidth - 16);
  const top = Math.max(12, Math.min(y - 18, window.innerHeight - 188));
  const style = {
    "--tooltip-anchor": placeRight ? "left" : "right",
    "--tooltip-x": `${left}px`,
    "--tooltip-y": `${top}px`,
    "--tooltip-color": color ?? "var(--accent)",
  } as CSSProperties;

  return (
    <div className="chart-tooltip-floating" data-anchor={placeRight ? "left" : "right"} style={style}>
      <div className="tooltip-title">
        <span />
        <strong>{title}</strong>
      </div>
      <div className="tooltip-values">
        <span>{primary}</span>
        {secondary ? <em>{secondary}</em> : null}
      </div>
      {lines.length ? (
        <div className="tooltip-lines">
          {lines.map((line) => (
            <div key={`${line.label}-${line.value}`}>
              <span style={{ "--line-color": line.color ?? "var(--tooltip-color)" } as CSSProperties} />
              <strong>{line.label}</strong>
              <em>{line.value}</em>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
