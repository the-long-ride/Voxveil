const FORBIDDEN_LICENSE = /unknown|non-commercial|research|\bnc\b/i;

export function approvalErrors(name, approved) {
  const errors = [];
  if (approved.commercialUse !== true) errors.push(`${name} is not approved for commercial use`);
  if (!approved.license || FORBIDDEN_LICENSE.test(approved.license)) {
    errors.push(`${name} has unacceptable license metadata: ${approved.license ?? 'missing'}`);
  }
  return errors;
}
