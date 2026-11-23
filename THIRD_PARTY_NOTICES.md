# Third-party notices

Driver-source notices remain in `native/windows/driver/NOTICE.md` and
`third_party/microsoft/windows-driver-samples`.

## Optional AI models (downloaded only with user consent)

### Spleeter 2-stems FP16

Copyright (c) 2019-present, Deezer SA.

Spleeter by Deezer Research: Romain Hennequin, Anis Khlif, Felix Voituret and
Manuel Moussallam. The [upstream paper](https://github.com/deezer/spleeter/blob/master/paper.md)
explicitly distributes source and pretrained models under MIT; see the
[license](https://github.com/deezer/spleeter/blob/master/LICENSE).

FP16 ONNX conversion by the sherpa-onnx/k2-fsa contributors, using
[Apache-2.0-licensed export tooling](https://github.com/k2-fsa/sherpa-onnx/tree/master/scripts/spleeter).
The export tooling is not bundled. Its [license](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE)
is separate from Deezer's MIT weights license.

### UVR MDX Inst HQ3

Credit: Ultimate Vocal Remover (UVR), Anjok07 and aufr33; MDX-Net architecture
by KUIELab / Woosung Choi. UVR-MDX-NET-Inst_HQ_3 weights provided by UVR.

The [UVR license statement](https://github.com/Anjok07/ultimatevocalremovergui/blob/a5f88453bfb2b38b05a965bcf67727243e0cbf19/README.md#license)
asks third-party developers using its models to honor MIT and credit UVR and
its developers. This approval covers the catalog's exact UVR-team HQ3 artifact,
not arbitrary third-party weights in UVR's download center. The publisher's
[pinned upload](https://huggingface.co/Anjok/model-repo/tree/048ddcf9a0db28453d71943c1e18a1e25291a642)
and SHA-256 are recorded in `tauri/models/catalog.json`.

## Inference runtime and Rust audio dependencies

ONNX Runtime 1.23.2 CPU: Copyright (c) Microsoft Corporation, MIT.
Windows packages include `ONNX_RUNTIME_LICENSE.txt` and the complete upstream
`ONNX_RUNTIME_ThirdPartyNotices.txt` from the hash-pinned Microsoft NuGet package.
`onnxruntime.dll` and `onnxruntime_providers_shared.dll` are app-local libraries,
not system drivers. No model weights are bundled.

`ort` 2.0.0-rc.11 and `rustfft` 6.4.1: MIT OR Apache-2.0.
`rubato` 0.16.2: MIT, Copyright (c) 2020 Henrik Enquist.
See their crate distributions for full copyright and license notices.

## MIT permission and warranty notice

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
