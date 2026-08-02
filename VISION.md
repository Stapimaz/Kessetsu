# NETLANG - CORE VISION & STRATEGY

## The Ultimate Goal
NetLang is **NOT** just another schematic drawing tool or web app.
The ultimate goal of NetLang is to be a **Code-to-Circuit Framework for Agentic LLMs**.

In the future, a user will prompt an LLM: "Design a 3-stage amplifier with X gain and Y passband."
The LLM will:
1. Write NetLang Micro-DSL code (`amp.nl`).
2. Run the NetLang CLI in the terminal (`netlang-cli amp.nl`).
3. Read the DRC (Self-Healing) errors from `stderr` or read the SPICE netlist.
4. If there are DRC errors, the LLM will autonomously fix its `.nl` code and retry.
5. If DRC passes, the LLM will run `ngspice` on the generated `.spice` file to check the gain/passband.
6. Iterate until the electronic specifications are met.
7. Present the final, mathematically proven circuit to the user, who can view the schematic via the Web UI (WASM).

## Development Roadmap
To achieve this, the architecture must be developed in the following strict order:
1. **MVP Core (Done)**: Parser, Graph Netlist, SPICE Generator, WASM UI.
2. **Modularity (Done)**: `module` and `use` keywords for scalable, hierarchical design.
3. **Core Expansion (Next)**: The core currently only knows Resistors, Capacitors, and Batteries. It must understand Transistors (BJT/MOSFET), Diodes, OpAmps, and Inductors to build an amplifier.
4. **CLI (Command Line Interface)**: Wrap the Rust core in a native binary so LLMs can run it from a terminal.
5. **Simulation Loop**: Build the bridge (or guidelines) for LLMs to run ngspice on the generated netlists.
6. **KiCad Export**: Generate physical PCB layout (`.kicad_pcb`) files from the graph.

**Note to future LLM agents:** Always check this file before proposing architectural changes. Do not deviate from making the DSL and error outputs "Agent-Friendly".
