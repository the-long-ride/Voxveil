import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { RangeControl } from './RangeControl';

describe('RangeControl', () => {
  it('supports optional labels and emits numeric values', () => {
    const onChange = vi.fn();
    render(<RangeControl id="quality-test" label="Quality" value={50} startLabel="Fast" endLabel="Fine" valueLabel="50%" onChange={onChange} />);
    expect(screen.getByText('Fast')).toBeInTheDocument();
    expect(screen.getByText('Fine')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Quality'), { target: { value: '70' } });
    expect(onChange).toHaveBeenCalledWith(70);
  });

  it('omits optional metadata when not provided', () => {
    render(<RangeControl id="plain" label="Plain" value={0} onChange={() => undefined} />);
    expect(screen.queryByText('%')).not.toBeInTheDocument();
  });
});
