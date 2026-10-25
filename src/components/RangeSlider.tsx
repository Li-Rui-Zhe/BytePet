import type { CSSProperties, InputHTMLAttributes } from "react";

interface RangeSliderProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "min" | "max"> {
  label: string;
  value: number;
  min: number;
  max: number;
}

/** Native range semantics and keyboard support, with one shared themed appearance. */
export function RangeSlider({ label, min, max, value, className = "", style, ...props }: RangeSliderProps) {
  const progress = Number.isFinite(value) && max > min
    ? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100))
    : 0;
  return <input {...props} type="range" aria-label={label} min={min} max={max} value={value}
    className={`ui-range ${className}`.trim()}
    style={{ ...style, "--range-progress": `${progress}%` } as CSSProperties} />;
}
