interface RangeControlProps {
  id: string;
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  startLabel?: string;
  endLabel?: string;
  valueLabel?: string;
  onChange: (value: number) => void;
}

export function RangeControl(props: RangeControlProps) {
  const { id, label, value, min = 0, max = 100, step = 1, startLabel, endLabel, valueLabel, onChange } = props;
  return (
    <div className="range-control">
      <div className="control-heading">
        <label htmlFor={id}>{label}</label>
        {valueLabel ? <span className="mono meta">{valueLabel}</span> : null}
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
      />
      {(startLabel || endLabel) ? (
        <div className="range-labels"><span>{startLabel}</span><span>{endLabel}</span></div>
      ) : null}
    </div>
  );
}
