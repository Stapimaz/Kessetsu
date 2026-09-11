# OPA197 Manufacturer-Model Probe — 2026-09-11

Status: authentic model acquired and integrity-checked for a local capability probe; not committed or redistributed. This is evaluator/model evidence, not an agent-design trial and not hardware validation.

## Provenance and permitted-use boundary

- Manufacturer/product: Texas Instruments OPA197.
- Official product page: <https://www.ti.com/product/OPA197>
- Official artifact: `OPAx197 PSpice Model (Rev. D)`, `SBOMA34D.ZIP`, downloaded from <https://www.ti.com/lit/zip/SBOMA34>.
- ZIP SHA-256: `9e55fcaa23d54cee025dda3fa9872b11c41c531f82e0802c2b93d51e943666e5`.
- Model file: `OPAx197.LIB`; SHA-256: `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`.
- Header identity: Texas Instruments copyright 2022; part `OPAx197`; date `23JUN2022`; model version `Final 1.3`; five-pin subcircuit order `IN+ IN- VCC VEE OUT`.
- The model header describes the file as a customer design aid, supplied as-is, and retains TI copyright. TI's current [Online Terms of Use](https://www.ti.com/legal/terms-conditions/terms-of-use.html) govern downloaded software/content and do not provide a clear basis for bundling this copyrighted file in Kessetsu's AGPL/commercial distribution. Therefore Kessetsu does not contain or redistribute the model. Any future commercial distribution needs an explicit compatible license/permission review. The current evaluator accepts only a user-supplied local file with the exact recorded hash and preserves its notices.

## Capability result

The exact model was tested with the repository's bundled Ngspice 46 and an evaluator-owned non-inverting gain-five circuit at ±6 V. The first probe exposed two integration mistakes rather than a model limitation: `ngbehavior=ps` translates PSpice syntax only in files read through `.include`, while the evaluator had embedded the model text in its main deck; and the deliberately minimized Windows runtime omitted the XSPICE transfer/switch modules needed by the translated `VSWITCH` elements.

The evaluator now writes the already hash-verified model to its isolated temporary directory, references it with `.include OPAx197.LIB`, and runs Ngspice in `ps` library mode. Kessetsu's Windows sidecar now contains only the required upstream `analog.cm` and `xtradev.cm` modules from the same official Ngspice 46 archive. The temporary model copy remains byte-for-byte identical, is deleted with the evaluator directory, and is omitted from evidence output except for its SHA-256 and provenance; the user's source file is untouched.

Result: `PASS`. The authentic TI model produced complete OP, 10 Hz–1 MHz AC, and 10 ms transient datasets; the evaluator's fixed gain-five AC and transient acceptance checks passed. The optional official-model test also verifies the exact hash, authentic-model flag, and absence of copyrighted model text from recorded evidence. This is simulator/model interoperability evidence, not hardware validation or permission to redistribute the TI file.
