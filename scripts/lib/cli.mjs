export function readNamedArgs(argv, names) {
  const result = {};
  for (const name of names) {
    const flag = `--${name}`;
    const index = argv.indexOf(flag);
    const value = index >= 0 ? argv[index + 1] : undefined;
    if (!value || value.startsWith('--')) throw new Error(`Missing ${flag}`);
    result[name] = value;
  }
  return result;
}
