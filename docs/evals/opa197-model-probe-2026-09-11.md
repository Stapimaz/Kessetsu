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

The exact model was tested with the repository's bundled Ngspice 46 using `ngbehavior=ps` and an evaluator-owned non-inverting gain-five circuit at ±6 V. Parsing still failed on the PSpice `IF` function (`no such function 'if'`), after switch-model compatibility warnings. No OP, AC, or transient dataset was produced.

Result: `U6_MODEL_CAPABILITY_ERROR`. Per the frozen U6 rules, this is the correct outcome—not a failed electrical design and not permission to substitute `KESSETSU_OPAMP_V1`. The U6 evaluator separately verifies fixed sources/load/topology, exact model hash/pin identity, successful metric behavior with a synthetic test-only compatibility fixture, and acquisition/capability failure records. Supporting the authentic model requires a licensed compatible model, a reviewed deterministic adapter, or another simulator backend; that blocker must be scoped before comparison runs.
