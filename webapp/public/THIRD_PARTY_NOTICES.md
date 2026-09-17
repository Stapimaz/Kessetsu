# Kessetsu Web Runtime — Third-Party Notices

## Threshold memristor example (unreleased)

Development builds offer the extracted Ngspice threshold-memristor library as an explicitly
downloadable example, not an automatically bound model. It retains BSD-3-Clause terms
and Pershin/Di Ventra/Holger Vogt attribution. The accompanying downloadable
`MEMRISTOR_NOTICE.md` contains origin, extraction changes and the complete notice.
The catalog documents its artificial coefficients and physical-fidelity limits.

## Inter

- Source: <https://github.com/rsms/inter>
- Distribution: `@fontsource-variable/inter@5.3.0`
- License: SIL Open Font License 1.1

The Web Hub ships a 2.2 KiB subset containing only the lowercase Kessetsu wordmark glyphs. The full license is shipped at `licenses/inter-OFL.txt` and remains available through the pinned package.

## EEcircuit Engine 1.7.0

- Source: <https://github.com/eelab-dev/EEcircuit-engine>
- License: MIT
- Copyright: 2024 EElab.dev
- npm integrity: `sha512-ZDpr/w/H81uCH3n2vjf0vohxOQqQ4NCsvaXkoYwcH+LCxIGKpBdvAIvGN5IdozW6AFJ8tojquKvDya3337yjSQ==`

The complete MIT text is shipped at `licenses/eecircuit-engine-MIT.txt`.

## Ngspice

EEcircuit Engine embeds Ngspice compiled to WebAssembly. The runtime currently reports `ngspice-45.2+`. Ngspice code is predominantly distributed under the modified BSD (BSD-3-Clause) license, with component-specific notices described by the upstream project.

- Project and license information: <https://ngspice.sourceforge.io/devel.html>
- FAQ/legal information: <https://ngspice.sourceforge.io/faq.html>

The full upstream Ngspice licensing inventory is shipped at `licenses/ngspice-COPYING.txt`. This summary does not replace that inventory.
