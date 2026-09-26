# Web Editor Guide

Development source adds **Analyze → Parameter study…** for multi-condition runs,
tolerances, comparison and reports; see [parameter studies](parameter-studies.md).
Published 1.2.0 does not include this menu yet.

Open the [Web Hub](https://kessetsu.com/#editor). No account or installation is required.
Compilation and simulation run locally in your browser; circuit source and model files
are not uploaded. See the [tutorial](tutorial.md) for a complete first circuit.

## Start or open a circuit

Use **File → Examples** for an RC filter, gain stage, power amplifier, reusable blocks
or local-model examples. **File → New circuit** starts a document; **Open .kess…** reads
a local source file. Rename the document through **File → Rename…** or the Share dialog.
Example selection replaces the active source; save work before switching.

Development source also provides **File → Import SPICE netlist…** for `.cir`, `.sp`, `.spice`
and `.net` files in the [declared import subset](spice-import.md). A successful conversion opens
ordinary editable `.kess` as an unsaved document. Unsupported content reports original line
numbers and leaves the current circuit in place. You can also drop a `.kess` or supported SPICE
file onto the workspace; the same validation and unsaved-work confirmation apply.

The Source panel is a text editor with language completion and diagnostics. Changes
automatically compile, run ERC and update the schematic after a short delay. The header
status reports checking, success or errors. Fix source diagnostics before exporting or running.
An automatic check is structural verification, not a completed simulation.

## Arrange the panels

Drag the dividers to resize Source, Schematic and Simulation. Each panel header has
minimize and maximize controls; minimized panels can be restored. Use
**View → Reset panel layout** to return to the default arrangement.

In Schematic, scroll to zoom around the pointer, drag to pan, and use Fit to see the whole
circuit. Toggle the dotted grid in its header. Select a component to highlight its symbol,
reference and value. The inspector lists its value/model provenance and maps each logical pin to
the canonical net; selecting a pin follows and highlights that complete net. Use Find to search
components or named nets and center them in the view. Reusable module instances appear as
navigable groups while electrical compilation and simulation remain canonically flattened.
The drawing's connectivity check concerns the schematic, not hardware.

## Run and inspect simulation

Write the analyses you need (`simulate op`, `tran`, `ac` or `dc`) in source. Press
**Run simulation** in the Simulation panel. The simulator loads on the first run; progress
and cancellation are shown there. Errors are not replaced with invented results.

Select an analysis and signal to inspect operating-point values or waveform/Bode/DC plots.
Hover over a plot for numeric values. Assertions show PASS, FAIL, ERROR or SKIPPED with their
measured value and limit. Simulation without assertions is valid but does not prove requirements.
When the source records physical-part ratings, a separate **Provided part limits** table shows
model stress, utilization, recorded conditions and citation. Its Within/Exceeds/Unavailable state
is advisory and never presented as a requirement PASS or hardware-safety approval.
Editing the source makes previous results stale: rerun before drawing conclusions.
See [simulation and assertions](../reference/simulation-and-assertions.md) and
[measurements](../reference/measurements.md) for exact meanings and analysis requirements.

## Review a proposal from an external agent

Development source provides **Analyze → Review agent proposal…** as a provider-neutral handoff.
Describe the outcome you own, then copy or download the versioned agent-task JSON. You decide which
external agent receives it; Kessetsu does not upload the circuit, contact a model provider or store
an API key. The task binds the circuit name, exact source and source SHA-256 to the requirements.

Load the returned `kessetsu.agent-proposal.v1` JSON in the same dialog. Kessetsu rejects proposals
for another source revision and keeps an accepted proposal's explanation separate from engineering
evidence. The proposed source is shown as a line diff and compiled through the same local Core while
the editor document remains unchanged. Run the proposal to obtain real local simulation and
assertion results. **Accept proposal** becomes available only after that run completes; closing the
dialog or stopping the run changes nothing. After acceptance, **Analyze → Undo accepted proposal**
restores the exact prior source as long as it has not since been edited.

The proposal summary is untrusted text. Compilation verifies supported syntax and schematic
connectivity; simulation verifies only the modeled analyses and assertions in the proposed source.
Neither step proves hardware, hidden requirements or an agent's broader claims. Local model files
remain local and must already be bound for a proposal that depends on them.

## Save and recover

Saving depends on browser capabilities, not merely its brand:

- Browsers with the native file picker (typically Chromium): **Save / Ctrl+S** selects a
  destination on first use, then writes the same permitted file. **Save As / Ctrl+Shift+S**
  selects a different destination. Canceling the picker does not mark changes saved.
- Browsers without that picker (including Firefox): **Save in browser / Ctrl+S** stores the
  document locally and clears the unsaved indicator. **Download .kess / Ctrl+Shift+S** makes
  a portable file copy; repeated downloads may receive numbered filenames.

An unsaved browser draft is recovered when storage is available. It is not cloud backup:
clearing site data, private browsing or another browser/device can make it unavailable.
Keep a downloaded `.kess` copy for important work. Circuit tools preserve a previous circuit
for **File → Restore previous circuit** where available.

## Select a local device model

For an `external_subcircuit`, open **View → Circuit details…** and select the file beside
the declared resource. Its exact hash and interface must match before compilation succeeds.
Files remain in memory only: reselect after reload, opening another document or sharing.
Browser-incompatible models require the native CLI; there is no generic replacement.
The [model catalog](../reference/model-catalog.md) provides example downloads and limits.

## Export or share

Use **Export** in the main header and choose a format and its available options. Review
format warnings, especially model dependencies and EDA limitations. Save source separately;
a PNG is not an editable circuit. See [export formats](../reference/exports.md).

**Share** opens a dialog where you can name the circuit, then copy its URL. The link carries
source and exact package identities, not current simulation datasets, file permissions or
local model bytes. A recipient recompiles and runs locally. External models must be supplied
separately under their license terms. Large sources produce longer links; no server-stored
short-link service is currently provided.

For compilation, simulation or storage failures, consult [troubleshooting](troubleshooting.md).
