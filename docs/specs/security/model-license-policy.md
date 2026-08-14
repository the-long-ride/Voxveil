# AI Model License and Distribution Policy

Voxveil does not train models as a product requirement and does not need a model to provide Classic DSP processing.

A pretrained model is eligible for distribution only when all of the following are independently verified for the exact shipped artifact:

- implementation/code license permits commercial use;
- checkpoint/weight license permits commercial use;
- redistribution of the exact checkpoint is explicit;
- modification/conversion/quantization rights cover the intended package;
- no non-commercial, research-only, or education-only restriction exists;
- attribution/notice obligations are practical and are included in release materials;
- model provenance and cryptographic hash are recorded;
- model package contains no executable installer or network behavior.

Architecture license and checkpoint license are separate review items. A permissive repository license does not automatically license weights.

If any condition is ambiguous, the release gate is **reject**. Users may not be silently encouraged to download an unclear checkpoint from inside Voxveil.
