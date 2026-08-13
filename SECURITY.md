# Security Policy

## Supported release

Security fixes target the latest published Kessetsu release. The first release deliberately rejects raw SPICE directives, floating model-package versions, unknown share schemas and unverified export connectivity.

## Reporting a vulnerability

Do not open a public issue for an exploitable vulnerability. Use GitHub's private vulnerability reporting for this repository. Include the affected version, minimal reproduction, impact and any known mitigation. Never include credentials, private circuit designs or unrelated personal data.

## Data boundary

Web Hub compiles, simulates, exports and compresses share links in the browser. The first release has no accounts, analytics, error telemetry or project-storage API. A circuit is disclosed only when the user deliberately shares its URL fragment or downloaded artifact. Browser/runtime limitations and third-party notices remain documented in the repository.
